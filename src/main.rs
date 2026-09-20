use anyhow::{Context, Result, anyhow};
use bytes::{Buf, Bytes, BytesMut};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
};

const KEY_SIZE: usize = 8;
const VAL_SIZE: usize = 8;
const TOOMSTONE_SIZE: u8 = 1;

struct Log {
    path: PathBuf,
    fp: File,
}

impl Log {
    fn new(path: String) -> Result<Self> {
        let file_path = PathBuf::from(&path);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .mode(0o644)
            .open(&file_path)
            .context(format!("failed to open file {}", path))?;
        file.sync_all()
            .context(format!("{:?} sync failed", file_path))?;
        let Some(parent) = file_path.parent() else {
            return Err(anyhow!("can't find parent for {}", path));
        };
        let dir = File::open(parent).context(format!("failed to open {:?}", parent))?;
        dir.sync_all()
            .context(format!("{:?} sync failed", parent))?;
        Ok(Self {
            fp: file,
            path: file_path,
        })
    }

    fn write_all(&mut self, kv: &KV) -> Result<()> {
        let bytes: Bytes = kv.into();
        self.fp.write_all(&bytes).context("failed to write")?;
        self.fp.sync_all().context("file sync failed")
    }

    fn append(&mut self, key: &Bytes, val: &Bytes, toomstone: bool) -> Result<()> {
        let mut buf: Vec<u8> = Vec::with_capacity(key.len() + val.len() + TOOMSTONE_SIZE as usize);
        buf.extend_from_slice(&key.len().to_be_bytes());
        buf.extend_from_slice(&val.len().to_be_bytes());
        if toomstone {
            buf.extend_from_slice(&1u8.to_be_bytes())
        } else {
            buf.extend_from_slice(&0u8.to_be_bytes())
        }
        buf.extend_from_slice(&key);
        buf.extend_from_slice(&val);
        self.fp.write_all(&buf).context(format!(
            "failed to append {:?}:{:?} { }",
            key, val, toomstone
        ))?;
        self.fp.sync_all().context("sync after append failed")
    }
}

struct KV<L = Log> {
    mem: BTreeMap<Bytes, (Bytes, bool)>,
    loger: L,
}

impl KV {
    fn set(&mut self, key: Bytes, val: Bytes) -> Result<Option<(Bytes, bool)>> {
        let ok = self.mem.insert(key.clone(), (val.clone(), false));
        self.loger.append(&key, &val, false)?;
        Ok(ok)
    }

    fn get(&self, key: &Bytes) -> Option<&(Bytes, bool)> {
        self.mem.get(key)
    }

    fn delete(&mut self, key: Bytes) -> Result<Option<(Bytes, (Bytes, bool))>> {
        let Some((k, val)) = self.mem.remove_entry(&key) else {
            return Ok(None);
        };
        let (value, _) = val;
        let ok = self.mem.insert(k.clone(), (value.clone(), true));
        self.loger.append(&k, &value, true)?;
        Ok(Some((k, (value, true))))
    }
}

// Serialization
impl Into<Bytes> for &KV {
    fn into(self) -> Bytes {
        let mut buff = Vec::with_capacity(self.mem.iter().fold(0, |acc, (k, v)| {
            acc + k.len() + v.0.len() + TOOMSTONE_SIZE as usize
        }));
        for (key, val) in self.mem.iter() {
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
            buffer.extend_from_slice(&key);
            buffer.extend_from_slice(&(val.0));
            buff.extend_from_slice(&buffer)
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
            assert!(KEY_SIZE == 8); // to make sure get_u64 is upadted if size is chanegd
            let key_len = bytes.try_get_u64().context("not a valid u64 key length")? as usize;
            assert!(VAL_SIZE == 8); // to make sure get_u64 is upadted if size is chanegd
            let val_len = bytes
                .try_get_u64()
                .context("not a valid u64 value length ")? as usize;
            let toomstone = bytes.try_get_u8().context("not a valid u8 toomstone")? == 1;

            let key = bytes.copy_to_bytes(key_len);
            let val = bytes.copy_to_bytes(val_len);

            mem.insert(key, (val, toomstone));
        }
        Ok(Self { mem, loger: value })
    }
}

fn main() {
    println!("Hello, world!");
}
