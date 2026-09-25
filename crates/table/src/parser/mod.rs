use anyhow::{Result, bail};
use bytes::Bytes;

use crate::{ColumnType, Value};

pub enum Token {
    Select,
    Upadte,
    Delete,
    Inster,
    Create,
    From,
    Table(String),
    Columns(Vec<ColumnType>), // SELECT col1,col2,col3
    Values(Vec<Vec<Value>>),  // INSER INTO table_name values (v1,v2,v3),...
    Set(String, Value),       // SET COL = VAL
    NULL,
    LT,
    GT,
    LTE,
    GTE,
    EQ,
    NQ,
    ADD,
    SUBSTRACT,
    MULTIPLAY,
    DEVIDE,
}
