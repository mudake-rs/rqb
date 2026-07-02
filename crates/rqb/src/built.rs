use sqlx::postgres::PgArguments;

use crate::Params;
use crate::Result;
use crate::summary::{format_query_sql, format_query_summary};

/// Rendered SQL plus its bind parameters.
///
/// Builders expose convenience `fetch_*` methods that call `build()` each time.
/// Keep a `BuiltQuery` when you want to inspect SQL, log the generated shape,
/// or execute the same validated query more than once with the same binds.
#[derive(Clone, Debug)]
#[must_use]
#[non_exhaustive]
pub struct BuiltQuery {
    /// Rendered Postgres SQL using `$N` placeholders.
    pub sql: String,
    /// Bind parameters in placeholder order.
    pub params: Params,
    /// Whether this query is safe to reuse as a stable prepared statement shape.
    ///
    /// rqb passes this to sqlx `.persistent(cacheable)`. SQLx keeps a bounded
    /// per-connection prepared-statement cache for Postgres, with a default
    /// capacity of 100 entries. Raw SQL fragments make the query non-cacheable
    /// because rqb cannot prove that their text is a stable statement shape.
    ///
    /// High-cardinality typed query shapes, such as many different `IN` list
    /// lengths or optional-filter combinations, stay memory-bounded by sqlx's
    /// LRU cache but can still churn prepared statements. Tune sqlx with
    /// `PgConnectOptions::statement_cache_capacity` or the
    /// `statement-cache-capacity` URL parameter when an application needs a
    /// different cache size.
    pub cacheable: bool,
}

impl BuiltQuery {
    /// Converts stored parameters into sqlx Postgres arguments for one execution.
    ///
    /// sqlx argument buffers are consumed by execution, so each execute/fetch
    /// path creates a fresh `PgArguments` value from the stored params.
    pub fn arguments(&self) -> Result<PgArguments> {
        self.params.arguments()
    }

    /// Returns debug-friendly query output.
    ///
    /// The output includes pretty-printed rendered SQL, bind parameter count,
    /// Rust type names, and cacheability. It never interpolates bind values
    /// into SQL.
    pub fn summary(&self) -> String {
        format_query_summary(self)
    }

    /// Returns only the formatted SQL text for human-readable logs.
    ///
    /// This does not change [`BuiltQuery::sql`], which remains the exact SQL
    /// text used for execution.
    pub fn pretty_sql(&self) -> String {
        format_query_sql(&self.sql)
    }
}

#[cfg(test)]
mod tests {
    use crate::{BuiltQuery, Param, Params};

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

            let owned_rows = built.clone().fetch_stream_pool(pool.clone()).unwrap();
            drop(owned_rows);

            let owned_typed_rows = built
                .clone()
                .fetch_stream_pool_as::<(i64,)>(pool.clone())
                .unwrap();
            drop(owned_typed_rows);

            let owned_scalars = built.fetch_stream_pool_scalar::<i64>(pool).unwrap();
            drop(owned_scalars);
        }

        let _ = assert_type_checks as fn(sqlx::PgPool, BuiltQuery);
    }
}
