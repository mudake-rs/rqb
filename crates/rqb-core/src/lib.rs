//! Core query model, metadata, validation, and serde-facing request types.
//!
//! `rqb-core` does not connect to a database. It owns the runtime-free pieces:
//! datasets, fields, expressions, write queries, `SearchRequest`, and runtime
//! validation against field metadata.
//!
//! `rqb-postgres` takes validated query models and renders or executes them.

mod aggregate;
mod builder;
mod dataset;
mod error;
mod expr;
mod field;
mod raw;
mod request;
mod serde_bridge;
mod sql_expr;
mod types;
mod validation;
mod value;
mod write;

pub use aggregate::{
    Aggregate, AggregateType, SelectColumn, array_agg, avg, count, count_distinct, count_field,
    json_agg, json_agg_nullable, max, max_agg, min, min_agg, string_agg, sum,
};
pub use builder::{SelectBuilder, select};
pub use dataset::{Cte, CteBody, Dataset, Join, JoinKind, Relation, Source, cte};
pub use error::{Error, Result};
pub use expr::{
    ColumnOperator, ColumnPredicate, ExistsPredicate, Expr, LogicalExpr, LogicalOp, NullsOrder,
    Operator, Predicate, Sort, SortDir, SubqueryOperator, SubqueryPredicate, all, any, exists,
    field, not, not_exists,
};
pub use field::{Capabilities, Field, FieldRef, JsonPathPolicy, ResolvedField, TextSearchConfig};
pub use raw::{RawQuery, RawSql, raw, raw_query};
pub use request::{LockMode, LockWait, RowLock, SearchRequest, SelectQuery};
pub use serde_bridge::fields_from_serializable;
pub use sql_expr::{
    CaseBranch, CaseBuilder, CaseThenBuilder, FunctionBuilder, IntoSqlExpr, SelectItem, SqlExpr,
    case_when, cast, coalesce, excluded, func, raw_expr,
};
pub use types::{
    DbEnum, ElemType, EnumType, FieldType, SelectRepr, TypeFamily, TypeSpec, ValueRepr,
    range_type_name,
};
pub use validation::{
    ValidatedAggregate, ValidatedArraySetOperator, ValidatedAssignment, ValidatedBinaryOperator,
    ValidatedConflictAction, ValidatedConflictClause, ValidatedConflictTarget,
    ValidatedContainmentOperator, ValidatedContainmentTarget, ValidatedCte, ValidatedCteBody,
    ValidatedDelete, ValidatedExpr, ValidatedInsert, ValidatedJoin, ValidatedLikePattern,
    ValidatedNullSafeBinaryOperator, ValidatedPredicate, ValidatedReturningItem, ValidatedSelect,
    ValidatedSelectItem, ValidatedSort, ValidatedSqlExpr, ValidatedUpdate, ValidatedWriteValue,
};
pub use value::Value;
pub use write::{
    ConflictAction, ConflictClause, ConflictTarget, DeleteBuilder, DeleteQuery, InsertBuilder,
    InsertConflictBuilder, InsertQuery, IntoFieldRefs, ReturningMode, UpdateBuilder, UpdateQuery,
    WriteAssignment, WriteValue, delete, insert, set, set_col, set_default, set_expr, set_raw,
    update,
};

pub mod prelude {
    pub use crate::{
        Aggregate, AggregateType, Capabilities, CaseBranch, CaseBuilder, CaseThenBuilder,
        ColumnOperator, ColumnPredicate, ConflictAction, ConflictClause, ConflictTarget, Cte,
        Dataset, DbEnum, DeleteBuilder, DeleteQuery, ElemType, EnumType, ExistsPredicate, Expr,
        Field, FieldRef, FieldType, FunctionBuilder, InsertBuilder, InsertConflictBuilder,
        InsertQuery, IntoSqlExpr, Join, JoinKind, JsonPathPolicy, LockMode, LockWait, LogicalOp,
        NullsOrder, Operator, RawQuery, RawSql, Relation, ReturningMode, RowLock, SearchRequest,
        SelectBuilder, SelectColumn, SelectItem, SelectQuery, SelectRepr, Sort, SortDir, SqlExpr,
        SubqueryOperator, SubqueryPredicate, TextSearchConfig, TypeFamily, TypeSpec, UpdateBuilder,
        UpdateQuery, Value, ValueRepr, WriteAssignment, WriteValue, all, any, array_agg, avg,
        case_when, cast, coalesce, count, count_distinct, count_field, cte, delete, excluded,
        exists, field, func, insert, json_agg, json_agg_nullable, max, max_agg, min, min_agg, not,
        not_exists, raw, raw_expr, raw_query, select, set, set_col, set_default, set_expr, set_raw,
        string_agg, sum, update,
    };
}
