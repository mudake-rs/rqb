#[derive(Debug, Clone)]
pub(crate) struct Relation {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) kind: RelationKind,
    pub(crate) columns: Vec<Column>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationKind {
    Table,
    View,
}

#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) name: String,
    pub(crate) const_name: String,
    pub(crate) meta_name: String,
    pub(crate) ty: ColumnType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColumnType {
    Known(KnownType),
    RawOnly { pg: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KnownType {
    Text,
    Bool,
    Int2,
    Int4,
    Int8,
    Float4,
    Float8,
    Numeric,
    Uuid,
    Date,
    Time,
    Timetz,
    Timestamp,
    Timestamptz,
    Interval,
    Json,
    Bytes,
    Inet,
    Cidr,
    Range(Box<KnownType>),
    Array(Box<KnownType>),
}
