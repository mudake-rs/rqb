//! Public facade for rqb.
//!
//! rqb is a sqlx-first Postgres query builder. Application code describes
//! server-owned query shape with typed fields, renders parameterized SQL, and
//! executes through any `sqlx::Executor<Database = Postgres>`.

pub use chrono;
pub use rqb_postgres as postgres;
pub use rqb_postgres::{
    Assignment, BoolExpr, BoolOp, BuiltQuery, DbErrorInfo, DbErrorPosition, Delete, ErasedParam,
    Error, Field, Insert, JsonKind, Meta, OpSet, OrderDirection, OrderItem, Param, Params, RawStmt,
    Result, SearchFilter, SearchOperator, SearchPredicate, SearchRequest, SearchSort, Select,
    SelectItem, SortDirection, Source, Stmt, Update, ValueExpr, ValueOp, delete_from, insert, raw,
    select, table, update, view,
};
pub use serde;
pub use serde_json;
pub use sqlx;
pub use uuid;

pub mod typed {
    pub use rqb_postgres::typed::{
        Assignment, BoolExpr, BoolOp, BuiltQuery, Delete, ErasedParam, Field, Insert, JsonKind,
        Meta, OpSet, OrderDirection, OrderItem, Param, Params, RawStmt, SearchFilter,
        SearchOperator, SearchPredicate, SearchRequest, SearchSort, Select, SelectItem,
        SortDirection, Source, Stmt, Update, ValueExpr, ValueOp, delete_from, insert, raw, select,
        table, update, view,
    };
}

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
}
