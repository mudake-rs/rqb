//! sqlx-first Postgres query builder for Rust services.
//!
//! Application code describes server-owned query shape with typed fields,
//! renders parameterized SQL, and executes through any
//! `sqlx::Executor<Database = Postgres>`.

#![allow(clippy::result_large_err)]

mod error;
mod tx;
pub mod typed;

pub use chrono;
pub use error::{DbErrorInfo, DbErrorPosition, Error};
pub use serde;
pub use serde_json;
pub use sqlx;
pub use typed::{
    Assignment, BoolExpr, BoolOp, BuiltQuery, Delete, ErasedParam, Field, Insert, JsonKind, Meta,
    OpSet, OrderDirection, OrderItem, Param, Params, RawStmt, SearchFilter, SearchOperator,
    SearchPredicate, SearchRequest, SearchSort, Select, SelectItem, SortDirection, Source, Stmt,
    Update, ValueExpr, ValueOp, delete_from, insert, raw, select, table, update, view,
};
pub use uuid;

pub type Result<T> = std::result::Result<T, Error>;

pub mod prelude {
    pub use crate::{
        Assignment, BoolExpr, BoolOp, BuiltQuery, DbErrorInfo, DbErrorPosition, Delete,
        ErasedParam, Error, Field, Insert, JsonKind, Meta, OpSet, OrderDirection, OrderItem, Param,
        Params, RawStmt, Result, SearchFilter, SearchOperator, SearchPredicate, SearchRequest,
        SearchSort, Select, SelectItem, SortDirection, Source, Stmt, Update, ValueExpr, ValueOp,
        delete_from, insert, raw, select, table, update, view,
    };
}

#[cfg(test)]
mod tests {
    use super::{Error, Field, Meta, OpSet};

    #[test]
    fn facade_exports_typed_field_and_error() {
        static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::equality());
        const ID: Field<i32> = Field::new(&ID_META);

        let error = ID.gt(10).validate().unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidTypedOperator { field, operator }
                if field == "id" && operator == "gt"
        ));
    }

    #[test]
    fn tx_macro_type_checks_with_pool_reference_and_value_return() {
        fn assert_type_checks(pool: sqlx::PgPool) {
            let future = crate::tx!(&pool, |conn| {
                let _: &mut sqlx::PgConnection = conn;
                Ok::<_, Error>(123_i32)
            });
            drop(future);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool);
    }

    #[test]
    fn tx_macro_type_checks_with_owned_pool_expression() {
        fn assert_type_checks(pool: sqlx::PgPool) {
            let future = crate::tx!(pool.clone(), |conn| {
                let _: &mut sqlx::PgConnection = conn;
                Ok::<_, Error>(())
            });
            drop(future);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool);
    }
}
