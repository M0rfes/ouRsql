use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const KEY_SIZE: usize = 8;
pub const VAL_SIZE: usize = 8;
pub const TOOMSTONE_SIZE: u8 = 1;
pub const TOMBSTONE_SIZE: u8 = TOOMSTONE_SIZE;

pub struct Log {
    pub path: PathBuf,
    pub fp: File,
}

impl Log {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let file_path = path.into();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .mode(0o644)
            .open(&file_path)
            .context(format!("failed to open file {:?}", file_path))?;
        file.sync_all()
            .context(format!("{:?} sync failed", file_path))?;

        let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
        let parent = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        let dir = File::open(parent).context(format!("failed to open {:?}", parent))?;
        dir.sync_all()
            .context(format!("{:?} sync failed", parent))?;

        Ok(Self {
            fp: file,
            path: file_path,
        })
    }

    pub fn write_all(&mut self, kv: &KV) -> Result<()> {
        let bytes: Bytes = kv.into();
        self.fp.write_all(&bytes).context("failed to write")?;
        self.fp.sync_all().context("file sync failed")
    }

    pub fn append(&mut self, key: &Bytes, val: &Bytes, toomstone: bool) -> Result<()> {
        let mut buf: Vec<u8> = Vec::with_capacity(key.len() + val.len() + TOOMSTONE_SIZE as usize);
        buf.extend_from_slice(&key.len().to_be_bytes());
        buf.extend_from_slice(&val.len().to_be_bytes());
        if toomstone {
            buf.extend_from_slice(&1u8.to_be_bytes())
        } else {
            buf.extend_from_slice(&0u8.to_be_bytes())
        }
        buf.extend_from_slice(key);
        buf.extend_from_slice(val);
        self.fp.write_all(&buf).context(format!(
            "failed to append {:?}:{:?} {}",
            key, val, toomstone
        ))?;
        self.fp.sync_all().context("sync after append failed")
    }
}

pub struct KV<L = Log> {
    pub mem: BTreeMap<Bytes, (Bytes, bool)>,
    pub loger: L,
}

impl KV<Log> {
    pub fn new(loger: Log) -> Self {
        Self {
            mem: BTreeMap::new(),
            loger,
        }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let log = Log::new(path)?;
        KV::try_from(log)
    }

    pub fn set(&mut self, key: Bytes, val: Bytes) -> Result<Option<(Bytes, bool)>> {
        let ok = self.mem.insert(key.clone(), (val.clone(), false));
        self.loger.append(&key, &val, false)?;
        Ok(ok)
    }

    pub fn get(&self, key: &Bytes) -> Option<&(Bytes, bool)> {
        self.mem.get(key)
    }

    pub fn delete(&mut self, key: Bytes) -> Result<Option<(Bytes, (Bytes, bool))>> {
        let Some((k, val)) = self.mem.remove_entry(&key) else {
            return Ok(None);
        };
        let (value, _) = val;
        let _ok = self.mem.insert(k.clone(), (value.clone(), true));
        self.loger.append(&k, &value, true)?;
        Ok(Some((k, (value, true))))
    }
}

// Serialization
impl From<&KV> for Bytes {
    fn from(kv: &KV) -> Self {
        let mut buff = Vec::with_capacity(kv.mem.iter().fold(0, |acc, (k, v)| {
            acc + k.len() + v.0.len() + TOOMSTONE_SIZE as usize
        }));
        for (key, val) in kv.mem.iter() {
            let mut buffer = Vec::with_capacity(
                KEY_SIZE + VAL_SIZE + TOOMSTONE_SIZE as usize + key.len() + val.0.len(),
            );
            buffer.extend_from_slice(&key.len().to_be_bytes());
            buffer.extend_from_slice(&val.0.len().to_be_bytes());
            if val.1 {
                buffer.extend_from_slice(&[1])
            } else {
                buffer.extend_from_slice(&[0])
            }
            buffer.extend_from_slice(key);
            buffer.extend_from_slice(&val.0);
            buff.extend_from_slice(&buffer);
        }
        Bytes::from(buff)
    }
}

impl TryFrom<Log> for KV {
    type Error = anyhow::Error;

    fn try_from(value: Log) -> std::result::Result<Self, Self::Error> {
        let bytes = fs::read(&value.path).context(format!("failed to read {:?}", value.path))?;

        let mut bytes = Bytes::from(bytes);
        let mut mem = BTreeMap::new();
        while bytes.len() >= KEY_SIZE + VAL_SIZE + TOOMSTONE_SIZE as usize {
            assert!(KEY_SIZE == 8); // to make sure get_u64 is updated if size is changed
            let key_len = bytes.try_get_u64().context("not a valid u64 key length")? as usize;
            assert!(VAL_SIZE == 8); // to make sure get_u64 is updated if size is changed
            let val_len = bytes
                .try_get_u64()
                .context("not a valid u64 value length")? as usize;
            let toomstone = bytes.try_get_u8().context("not a valid u8 toomstone")? == 1;

            let key = bytes.copy_to_bytes(key_len);
            let val = bytes.copy_to_bytes(val_len);

            mem.insert(key, (val, toomstone));
        }
        Ok(Self { mem, loger: value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_set_get_delete() -> Result<()> {
        let dir = tempdir()?;
        let log_path = dir.path().join("test.log");

        let log = Log::new(log_path)?;
        let mut kv = KV::new(log);

        let k1 = Bytes::from("key1");
        let v1 = Bytes::from("val1");

        kv.set(k1.clone(), v1.clone())?;
        assert_eq!(kv.get(&k1), Some(&(v1.clone(), false)));

        let deleted = kv.delete(k1.clone())?;
        assert!(deleted.is_some());
        assert_eq!(kv.get(&k1), Some(&(v1, true))); // tombstoned

        Ok(())
    }

    #[test]
    fn test_persistence_and_recovery() -> Result<()> {
        let dir = tempdir()?;
        let log_path = dir.path().join("wal.log");

        let k1 = Bytes::from("user");
        let v1 = Bytes::from("alice");
        let k2 = Bytes::from("city");
        let v2 = Bytes::from("paris");

        // First session: write data
        {
            let mut kv = KV::open(&log_path)?;
            kv.set(k1.clone(), v1.clone())?;
            kv.set(k2.clone(), v2.clone())?;
            kv.delete(k2.clone())?;
        }

        // Second session: recover from log
        {
            let kv = KV::open(&log_path)?;
            assert_eq!(kv.get(&k1), Some(&(v1, false)));
            assert_eq!(kv.get(&k2), Some(&(v2, true)));
        }

        Ok(())
    }
}
