pub mod db;
pub mod parser;
use anyhow::{Result, ensure};
use bytes::{Buf, Bytes};
pub use db::DB;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    I64(i64),
    Str(Bytes),
}

pub type Row = Vec<Value>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    I64,
    Str,
}

pub struct Column {
    pub name: String,
    pub r#type: ColumnType,
}

pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub pk: Vec<usize>,
}

impl Table {
    pub fn get_rows(&self) -> Row {
        self.columns
            .iter()
            .map(|col| match col.r#type {
                ColumnType::I64 => Value::I64(0),
                ColumnType::Str => Value::Str(Bytes::new()),
            })
            .collect()
    }

    pub fn encode_key(&self, row: &Row) -> Bytes {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.extend_from_slice(self.name.as_bytes());
        buffer.push(0x00);
        for &i in self.pk.iter() {
            match &row[i] {
                Value::I64(i64) => {
                    buffer.extend_from_slice(&i64.to_le_bytes());
                }
                Value::Str(s) => {
                    buffer.extend_from_slice(&(s.len() as u64).to_le_bytes());
                    buffer.extend_from_slice(s);
                }
            }
        }
        Bytes::from(buffer)
    }

    pub fn decode_key(&self, data: &mut Bytes, row: &mut Row) -> Result<()> {
        ensure!(
            data.len() >= self.name.len() + 1,
            "key too short for table prefix"
        );
        let prefix = data.copy_to_bytes(self.name.len());
        ensure!(
            prefix.as_ref() == self.name.as_bytes(),
            "table name mismatch"
        );
        ensure!(data.get_u8() == 0x00, "missing 0x00 delimiter");

        for &i in &self.pk {
            let col = &self.columns[i];
            match col.r#type {
                ColumnType::I64 => {
                    ensure!(data.len() >= 8, "truncated I64");
                    row[i] = Value::I64(data.try_get_i64_le()?);
                }
                ColumnType::Str => {
                    ensure!(data.len() >= 8, "truncated string len");
                    let len = data.try_get_u64_le()? as usize;
                    ensure!(data.len() >= len, "truncated string payload");
                    row[i] = Value::Str(data.copy_to_bytes(len));
                }
            }
        }
        Ok(())
    }

    pub fn encode_val(&self, row: &Row) -> Bytes {
        let mut buffer: Vec<u8> = Vec::new();
        for (i, val) in row.iter().enumerate() {
            if self.pk.contains(&i) {
                continue;
            }
            match val {
                Value::I64(i64) => {
                    buffer.extend_from_slice(&i64.to_le_bytes());
                }
                Value::Str(s) => {
                    buffer.extend_from_slice(&(s.len() as u64).to_le_bytes());
                    buffer.extend_from_slice(s);
                }
            }
        }
        Bytes::from(buffer)
    }

    pub fn decode_val(&self, mut data: &[u8], row: &mut Row) -> Result<()> {
        for (i, col) in self.columns.iter().enumerate() {
            if self.pk.contains(&i) {
                continue;
            }

            match col.r#type {
                ColumnType::I64 => {
                    ensure!(data.len() >= 8, "truncated I64");
                    row[i] = Value::I64(data.try_get_i64_le()?);
                }
                ColumnType::Str => {
                    ensure!(data.len() >= 8, "truncated string len");
                    let len = data.try_get_u64_le()? as usize;
                    ensure!(data.len() >= len, "truncated string payload");
                    row[i] = Value::Str(data.copy_to_bytes(len));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() -> Result<()> {
        let table = Table {
            name: "users".to_string(),
            columns: vec![
                Column {
                    name: "id".to_string(),
                    r#type: ColumnType::I64,
                },
                Column {
                    name: "name".to_string(),
                    r#type: ColumnType::Str,
                },
                Column {
                    name: "age".to_string(),
                    r#type: ColumnType::I64,
                },
            ],
            pk: vec![0], // id is primary key
        };

        let original_row = vec![
            Value::I64(42),
            Value::Str(Bytes::from("Alice")),
            Value::I64(30),
        ];

        let mut key_bytes = table.encode_key(&original_row);
        let mut val_bytes = table.encode_val(&original_row);

        let mut decoded_row = table.get_rows();
        table.decode_key(&mut key_bytes, &mut decoded_row)?;
        table.decode_val(&mut val_bytes, &mut decoded_row)?;

        assert_eq!(decoded_row, original_row);
        Ok(())
    }
}
