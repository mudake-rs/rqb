use sqlx::postgres::PgArguments;

use crate::Result;
use crate::typed::Params;

/// Rendered SQL plus its bind parameters.
#[derive(Clone, Debug)]
#[must_use]
pub struct BuiltQuery {
    /// Rendered Postgres SQL using `$N` placeholders.
    pub sql: String,
    /// Bind parameters in placeholder order.
    pub params: Params,
    /// Whether this query is safe to reuse as a stable prepared statement shape.
    pub cacheable: bool,
}

impl BuiltQuery {
    /// Converts stored parameters into sqlx Postgres arguments.
    pub fn arguments(&self) -> Result<PgArguments> {
        self.params.arguments()
    }
}

#[cfg(test)]
mod tests {
    use crate::typed::{BuiltQuery, Param, Params};

    #[test]
    fn built_query_arguments_delegate_to_stored_params() {
        let built = BuiltQuery {
            sql: "select $1".to_owned(),
            params: Params::from_vec(vec![Param::typed(1_i32)]),
            cacheable: true,
        };

        built.arguments().unwrap();
    }

    #[test]
    fn built_query_clone_keeps_sql_params_and_cacheability() {
        let built = BuiltQuery {
            sql: "select $1".to_owned(),
            params: Params::from_vec(vec![Param::typed("x".to_owned())]),
            cacheable: false,
        };
        let cloned = built.clone();

        assert_eq!(cloned.sql, "select $1");
        assert_eq!(cloned.params.len(), 1);
        assert!(!cloned.cacheable);
    }

    #[test]
    fn built_query_stream_methods_type_check() {
        fn assert_type_checks(pool: sqlx::PgPool, built: BuiltQuery) {
            let rows = built.fetch_stream(&pool).unwrap();
            drop(rows);

            let typed_rows = built.fetch_stream_as::<(i64,)>(&pool).unwrap();
            drop(typed_rows);

            let scalars = built.fetch_stream_scalar::<i64>(&pool).unwrap();
            drop(scalars);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool, BuiltQuery);
    }
}
