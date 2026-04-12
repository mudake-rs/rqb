//! Core query model, metadata, validation, and serde-facing request types.
//!
//! `rqb-core` does not connect to a database. It owns the portable pieces:
//! datasets, fields, expressions, write queries, `SearchRequest`, and runtime
//! validation against field metadata.
//!
//! Backend crates, such as `rqb-postgres`, take validated query models and
//! render or execute them.

mod aggregate;
mod builder;
mod dataset;
mod error;
mod expr;
mod field;
mod raw;
mod request;
mod serde_bridge;
mod validation;
mod value;
mod write;

pub use aggregate::{
    Aggregate, AggregateType, SelectColumn, array_agg, avg, count, count_distinct, count_field,
    max, max_agg, min, min_agg, string_agg, sum,
};
pub use builder::{SelectBuilder, select};
pub use dataset::{Cte, CteBody, Dataset, Join, JoinKind, Relation, Source, cte};
pub use error::{Error, Result};
pub use expr::{
    ColumnOperator, ColumnPredicate, ExistsPredicate, Expr, LogicalExpr, LogicalOp, NullsOrder,
    Operator, Predicate, Sort, SortDir, SubqueryOperator, SubqueryPredicate, all, any, exists,
    field, not, not_exists,
};
pub use field::{
    Capabilities, DbEnum, ElemType, EnumType, Field, FieldRef, FieldType, JsonPathPolicy,
    ResolvedField, TextSearchConfig,
};
pub use raw::{RawSql, raw};
pub use request::{LockMode, LockWait, RowLock, SearchRequest, SelectQuery};
pub use serde_bridge::fields_from_serializable;
pub use validation::{
    ValidatedAggregate, ValidatedAssignment, ValidatedConflictAction, ValidatedConflictClause,
    ValidatedConflictTarget, ValidatedDelete, ValidatedInsert, ValidatedSelect, ValidatedSort,
    ValidatedUpdate, ValidatedWriteValue, resolve_field, resolve_query_field,
    resolve_query_field_with_outer,
};
pub use value::Value;
pub use write::{
    ConflictAction, ConflictClause, ConflictTarget, DeleteBuilder, DeleteQuery, InsertBuilder,
    InsertConflictBuilder, InsertQuery, IntoFieldRefs, ReturningMode, UpdateBuilder, UpdateQuery,
    WriteAssignment, WriteValue, delete, insert, update,
};

pub mod prelude {
    pub use crate::{
        Aggregate, AggregateType, Capabilities, ColumnOperator, ColumnPredicate, ConflictAction,
        ConflictClause, ConflictTarget, Cte, Dataset, DbEnum, DeleteBuilder, DeleteQuery, ElemType,
        EnumType, ExistsPredicate, Expr, Field, FieldRef, FieldType, InsertBuilder,
        InsertConflictBuilder, InsertQuery, Join, JoinKind, JsonPathPolicy, LockMode, LockWait,
        LogicalOp, NullsOrder, Operator, RawSql, Relation, ReturningMode, RowLock, SearchRequest,
        SelectBuilder, SelectColumn, SelectQuery, Sort, SortDir, SubqueryOperator,
        SubqueryPredicate, TextSearchConfig, UpdateBuilder, UpdateQuery, Value, WriteAssignment,
        WriteValue, all, any, array_agg, avg, count, count_distinct, count_field, cte, delete,
        exists, field, insert, max, max_agg, min, min_agg, not, not_exists, raw,
        resolve_query_field_with_outer, select, string_agg, sum, update,
    };
}
