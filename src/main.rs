use anyhow::{Context, Result};
use bytes::{Buf, Bytes, BytesMut};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
};

const KEY_SIZE: usize = 8;
const VAL_SIZE: usize = 8;
const TOOMSTONE_SIZE: u8 = 1;

struct Log {
    path: String,
    fp: File,
}
impl Log {
    fn new(path: String) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o644)
            .open(path.clone())
            .context(format!("failed to open file {}", path))?;

        Ok(Self { fp: file, path })
    }

    fn write(&mut self, kv: KV) -> Result<()> {
        let bytes: Bytes = kv.into();
        self.fp.write_all(&bytes).context("failed to write")
    }

    fn read(&self) -> Result<KV> {
        let bytes = fs::read(self.path.as_str())?;
        let bytes = Bytes::from(bytes);
        let kv = KV::try_from(bytes).context(format!("{} not a valid db", self.path))?;
        Ok(kv)
    }
}

struct KV {
    pub mem: BTreeMap<Bytes, (Bytes, bool)>,
}

// Serialization
impl Into<Bytes> for KV {
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

// Deserialization
impl TryFrom<Bytes> for KV {
    type Error = anyhow::Error;

    fn try_from(mut value: Bytes) -> std::result::Result<Self, Self::Error> {
        let mut mem = BTreeMap::new();
        while value.len() >= KEY_SIZE + VAL_SIZE + TOOMSTONE_SIZE as usize {
            assert!(KEY_SIZE == 8); // to make sure get_u64 is upadted if size is chanegd
            let key_len = value.try_get_u64().context("not a valid u64 key length")? as usize;
            assert!(VAL_SIZE == 8); // to make sure get_u64 is upadted if size is chanegd
            let val_len = value
                .try_get_u64()
                .context("not a valid u64 value length ")? as usize;
            let toomstone = value.try_get_u8().context("not a valid u8 toomstone")? == 1;

            let key = value.copy_to_bytes(key_len);
            let val = value.copy_to_bytes(val_len);

            mem.insert(key, (val, toomstone));
        }

        Ok(Self { mem })
    }
}

fn main() {
    println!("Hello, world!");
}
