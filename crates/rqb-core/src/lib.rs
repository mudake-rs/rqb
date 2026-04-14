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
mod query;
mod raw;
mod request;
mod sql_expr;
mod types;
mod validation;
mod value;
mod write;
mod write_record;

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
pub use query::{
    QueryExpr, SetOperator, SetQuery, except, except_all, intersect, intersect_all, union,
    union_all,
};
pub use raw::{RawQuery, RawSql, raw, raw_query};
pub use request::{LockMode, LockWait, RowLock, SearchRequest, SelectQuery};
pub use sql_expr::{
    BuiltinFunction, CaseBranch, CaseBuilder, CaseThenBuilder, FunctionBuilder, FunctionNameStyle,
    IntoSqlExpr, JsonAccessPath, OffsetWindowFunctionBuilder, SelectItem, SqlExpr, WindowFunction,
    WindowFunctionBuilder, WindowSpec, case_when, cast, coalesce, date_trunc, dense_rank, excluded,
    func, gen_random_uuid, greatest, lag, lead, least, length, lower, now, nullif, partition_by,
    rank, raw_expr, row_number, trim, upper, window,
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
    ValidatedNullSafeBinaryOperator, ValidatedPredicate, ValidatedQueryExpr,
    ValidatedReturningItem, ValidatedSelect, ValidatedSelectItem, ValidatedSetQuery,
    ValidatedSetSort, ValidatedSort, ValidatedSource, ValidatedSqlExpr, ValidatedUpdate,
    ValidatedWriteValue,
};
pub use value::Value;
pub use write::{
    ConflictAction, ConflictClause, ConflictTarget, DeleteBuilder, DeleteQuery, InsertBuilder,
    InsertConflictBuilder, InsertQuery, IntoFieldRefs, ReturningMode, UpdateBuilder, UpdateQuery,
    WriteAssignment, WriteValue, delete, insert, set, set_col, set_default, set_expr, set_raw,
    update,
};
pub use write_record::{__RqbWriteRecordResult, __rqb_json_write_value, WriteRecord};

pub mod prelude {
    pub use crate::{
        Aggregate, AggregateType, Capabilities, CaseBranch, CaseBuilder, CaseThenBuilder,
        ColumnOperator, ColumnPredicate, ConflictAction, ConflictClause, ConflictTarget, Cte,
        Dataset, DbEnum, DeleteBuilder, DeleteQuery, ElemType, EnumType, ExistsPredicate, Expr,
        Field, FieldRef, FieldType, FunctionBuilder, InsertBuilder, InsertConflictBuilder,
        InsertQuery, IntoSqlExpr, Join, JoinKind, JsonPathPolicy, LockMode, LockWait, LogicalOp,
        NullsOrder, OffsetWindowFunctionBuilder, Operator, QueryExpr, RawQuery, RawSql, Relation,
        ReturningMode, RowLock, SearchRequest, SelectBuilder, SelectColumn, SelectItem,
        SelectQuery, SelectRepr, SetOperator, SetQuery, Sort, SortDir, SqlExpr, SubqueryOperator,
        SubqueryPredicate, TextSearchConfig, TypeFamily, TypeSpec, UpdateBuilder, UpdateQuery,
        Value, ValueRepr, WindowFunction, WindowFunctionBuilder, WindowSpec, WriteAssignment,
        WriteRecord, WriteValue, all, any, array_agg, avg, case_when, cast, coalesce, count,
        count_distinct, count_field, cte, date_trunc, delete, dense_rank, except, except_all,
        excluded, exists, field, func, gen_random_uuid, greatest, insert, intersect, intersect_all,
        json_agg, json_agg_nullable, lag, lead, least, length, lower, max, max_agg, min, min_agg,
        not, not_exists, now, nullif, partition_by, rank, raw, raw_expr, raw_query, row_number,
        select, set, set_col, set_default, set_expr, set_raw, string_agg, sum, trim, union,
        union_all, update, upper, window,
    };
}
