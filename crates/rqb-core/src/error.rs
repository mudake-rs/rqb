use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("unknown field `{field}` for dataset `{dataset}`")]
    UnknownField { dataset: String, field: String },

    #[error("ambiguous field `{field}`; qualify it with one of: {matches}")]
    AmbiguousField { field: String, matches: String },

    #[error("unknown dataset qualifier `{qualifier}`")]
    UnknownDatasetQualifier { qualifier: String },

    #[error("ambiguous dataset qualifier `{qualifier}`; matches: {matches}")]
    AmbiguousDatasetQualifier { qualifier: String, matches: String },

    #[error("dataset qualifier `{qualifier}` is used more than once in the query")]
    DuplicateDatasetQualifier { qualifier: String },

    #[error("field `{field}` is not selectable")]
    NotSelectable { field: String },

    #[error("field `{field}` is not sortable")]
    NotSortable { field: String },

    #[error("field `{field}` is not filterable")]
    NotFilterable { field: String },

    #[error("field `{field}` is not a JSONB field and cannot use nested path `{path}`")]
    NotJsonbPath { field: String, path: String },

    #[error("JSONB field `{field}` does not allow dynamic paths")]
    JsonbPathDenied { field: String },

    #[error("operator `{operator}` is not supported for field `{field}` of type `{field_type}`")]
    UnsupportedOperator {
        field: String,
        field_type: String,
        operator: String,
    },

    #[error(
        "column operator `{operator}` is not supported between `{left}` ({left_type}) and `{right}` ({right_type})"
    )]
    IncompatibleColumnTypes {
        left: String,
        left_type: String,
        right: String,
        right_type: String,
        operator: String,
    },

    #[error("invalid value for operator `{operator}` on field `{field}`: {message}")]
    InvalidValue {
        field: String,
        operator: String,
        message: String,
    },

    #[error("invalid enum value `{value}` for field `{field}`; allowed values: {allowed}")]
    InvalidEnumValue {
        field: String,
        value: String,
        allowed: String,
    },

    #[error("text search is not configured for field `{field}`")]
    TextSearchNotConfigured { field: String },

    #[error("empty logical expression `{logical}`")]
    EmptyLogical { logical: String },

    #[error("NOT expression must contain exactly one predicate")]
    InvalidNot,

    #[error("requested limit {requested} exceeds maximum limit {max}")]
    LimitExceeded { requested: u32, max: u32 },

    #[error("{kind} join against `{dataset}` requires an ON condition")]
    MissingJoinCondition { kind: String, dataset: String },

    #[error("raw SQL fragment has {placeholders} placeholders but {binds} bind values")]
    RawBindMismatch { placeholders: usize, binds: usize },

    #[error("insert has no values")]
    EmptyInsert,

    #[error("update has no assignments")]
    EmptyUpdate,

    #[error("delete without filter is not allowed")]
    DeleteWithoutFilter,

    #[error("expected serialized record to be an object: {message}")]
    ExpectedObject { message: String },

    #[error("serialization error: {message}")]
    SerdeError { message: String },

    #[error("insert rows must have the same fields")]
    InconsistentInsertFields,

    #[error("write queries support only table and view datasets")]
    UnsupportedWriteSource,

    #[error("insert conflict filter requires `on_conflict(...).do_update(...)`")]
    InvalidConflictFilter,

    #[error("selected field `{field}` must be present in GROUP BY or be aggregated")]
    UngroupedField { field: String },

    #[error("aggregate alias `{alias}` is used more than once")]
    DuplicateAggregateAlias { alias: String },

    #[error("selected output alias `{alias}` is used more than once")]
    DuplicateOutputAlias { alias: String },

    #[error("unknown aggregate alias `{alias}`")]
    UnknownAggregateAlias { alias: String },

    #[error("aggregate `{alias}` does not support ordered input")]
    AggregateOrderUnsupported { alias: String },

    #[error("aggregate `{aggregate}` does not support field `{field}` of type `{field_type}`")]
    UnsupportedAggregateField {
        aggregate: String,
        field: String,
        field_type: String,
    },

    #[error("select expression alias must not be empty")]
    EmptyExpressionAlias,

    #[error("expression `{expression}` has no inferable output type")]
    UnknownExpressionType { expression: String },

    #[error(
        "expression `{expression}` mixes incompatible output types `{left_type}` and `{right_type}`"
    )]
    IncompatibleExpressionTypes {
        expression: String,
        left_type: String,
        right_type: String,
    },

    #[error("subquery must select {expected} column(s), but selects {actual}")]
    InvalidSubquerySelection { expected: usize, actual: usize },
}
