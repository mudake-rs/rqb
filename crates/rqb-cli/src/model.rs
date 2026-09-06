use serde::Deserialize;

#[derive(Debug, Clone)]
pub(crate) struct SchemaModel {
    pub(crate) enums: Vec<PgEnum>,
    pub(crate) relations: Vec<Relation>,
}

#[derive(Debug, Clone)]
pub(crate) struct PgEnum {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) variants: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Relation {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) kind: RelationKind,
    pub(crate) columns: Vec<Column>,
    pub(crate) constraints: Vec<UniqueConstraint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationKind {
    Table,
    View,
    MaterializedView,
}

#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) name: String,
    pub(crate) const_name: String,
    pub(crate) ty: ColumnType,
    pub(crate) nullable: bool,
    pub(crate) generated: GeneratedKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeneratedKind {
    Virtual,
    None,
    Stored,
    IdentityAlways,
    IdentityByDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColumnType {
    Known(KnownType),
    Custom {
        pg: String,
        rust: String,
        array: bool,
        ops: FieldOps,
        json: Option<FieldJson>,
    },
    PgEnum {
        schema: String,
        name: String,
        pg: String,
        array: bool,
    },
    RawOnly {
        pg: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UniqueConstraint {
    pub(crate) name: String,
    pub(crate) const_name: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FieldOps {
    None,
    Equality,
    Ordered,
    Text,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FieldJson {
    Text,
    Bool,
    Integer,
    BigInt,
    Float,
    NumericString,
    Uuid,
    Date,
    Time,
    Timestamp,
    Timestamptz,
    Jsonb,
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
    Jsonb,
    Bytes,
    Range(Box<KnownType>),
    Array(Box<KnownType>),
}
