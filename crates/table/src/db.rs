use anyhow::Result;
use std::path::PathBuf;

use kv::KV;

use crate::{Row, Table};

pub struct DB {
    kv: KV,
}

impl DB {
    pub fn new(kv: KV) -> Self {
        Self { kv }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let kv = KV::open(path)?;
        Ok(Self { kv })
    }

    pub fn select(&self, table: &Table, row: &mut Row) -> Result<bool> {
        let key = table.encode_key(row);
        let Some(value) = self.kv.get(&key) else {
            return Ok(false);
        };

        table.decode_val(value, row)?;
        Ok(true)
    }

    pub fn insert(&mut self, table: &Table, row: &Row) -> Result<bool> {
        let key = table.encode_key(row);
        let value = table.encode_val(row);
        self.kv.set(key, value)
    }

    pub fn update(&mut self, table: &Table, row: &Row) -> Result<bool> {
        let key = table.encode_key(row);
        let value = table.encode_val(row);
        self.kv.update(key, value)
    }

    pub fn upsert(&mut self, table: &Table, row: &Row) -> Result<bool> {
        let key = table.encode_key(row);
        let value = table.encode_val(row);
        self.kv.upsert(key, value)
    }

    pub fn delete(&mut self, table: &Table, row: &Row) -> Result<bool> {
        let key = table.encode_key(row);
        self.kv.delete(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Column, ColumnType, Value};
    use bytes::Bytes;
    use tempfile::tempdir;

    fn sample_table() -> Table {
        Table {
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
            pk: vec![0],
        }
    }

    #[test]
    fn test_db_crud() -> Result<()> {
        let dir = tempdir()?;
        let log_path = dir.path().join("db.log");

        let mut db = DB::open(&log_path)?;
        let table = sample_table();

        let row1 = vec![
            Value::I64(1),
            Value::Str(Bytes::from("Alice")),
            Value::I64(30),
        ];

        // 1. Insert row1
        assert!(db.insert(&table, &row1)?);

        // 2. Duplicate insert should return false
        assert!(!db.insert(&table, &row1)?);

        // 3. Select row1 by PK
        let mut select_row = table.get_rows();
        select_row[0] = Value::I64(1);
        assert!(db.select(&table, &mut select_row)?);
        assert_eq!(select_row, row1);

        // 4. Update row1
        let updated_row1 = vec![
            Value::I64(1),
            Value::Str(Bytes::from("Alice")),
            Value::I64(31),
        ];
        assert!(db.update(&table, &updated_row1)?);

        let mut verify_update = table.get_rows();
        verify_update[0] = Value::I64(1);
        assert!(db.select(&table, &mut verify_update)?);
        assert_eq!(verify_update, updated_row1);

        // 5. Delete row1
        assert!(db.delete(&table, &row1)?);
        let mut verify_delete = table.get_rows();
        verify_delete[0] = Value::I64(1);
        assert!(!db.select(&table, &mut verify_delete)?);

        Ok(())
    }
}
