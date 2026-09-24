use bytes::Bytes;

pub enum Token {
    Select,
    Upadte,
    Delete,
    Inster,
    Create,
    From,
    Table(Bytes),
    Columns(Bytes),     // SELECT col1,col2,col3
    Values(Vec<Bytes>), // INSER INTO table_name values (v1,v2,v3),...
    Set(Bytes, Bytes),  // SET COL = VAL
    NULL,
    LT,
    GT,
    LTE,
    GTE,
    EQ,
    NQ,
}
