use std::fmt;

use sqlx::postgres::PgArguments;

use crate::Params;
use crate::Result;

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
    /// Raw SQL fragments make the query non-cacheable because rqb cannot prove
    /// that their text is a stable statement shape.
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

    /// Returns a display adapter for debug-friendly query output.
    ///
    /// The output includes the rendered SQL, bind parameter count and Rust type
    /// names, and cacheability. It never interpolates bind values into SQL.
    pub fn pretty(&self) -> PrettyQuery<'_> {
        pretty_query(self)
    }
}

/// Display adapter for debug-friendly [`BuiltQuery`] output.
///
/// Use [`BuiltQuery::pretty`] or [`pretty_query`] when logging generated SQL:
///
/// ```
/// rqb::schema! {
///     table public.users {
///         id: int4 = i32,
///     }
/// }
///
/// # fn main() -> rqb::Result<()> {
/// let built = rqb::select(users::table()).build()?;
/// println!("{}", built.pretty());
/// # Ok(())
/// # }
/// ```
#[must_use]
pub struct PrettyQuery<'a> {
    query: &'a BuiltQuery,
}

impl fmt::Display for PrettyQuery<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SQL:")?;
        writeln!(f, "{}", self.query.sql)?;
        writeln!(f)?;

        let params = self.query.params.as_slice();
        if params.is_empty() {
            writeln!(f, "Params: none")?;
        } else {
            writeln!(f, "Params ({}):", params.len())?;
            for (index, param) in params.iter().enumerate() {
                writeln!(f, "${}: {}", index + 1, param.debug_name())?;
            }
        }

        writeln!(f)?;
        write!(f, "Cacheable: {}", self.query.cacheable)
    }
}

/// Returns a display adapter for debug-friendly [`BuiltQuery`] output.
///
/// This is equivalent to [`BuiltQuery::pretty`].
#[inline]
pub fn pretty_query(query: &BuiltQuery) -> PrettyQuery<'_> {
    PrettyQuery { query }
}

#[cfg(test)]
mod tests {
    use crate::{BuiltQuery, Param, Params, pretty_query};

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
    fn pretty_query_formats_sql_params_and_cacheability() {
        let built = BuiltQuery {
            sql: "select $1, $2".to_owned(),
            params: Params::from_vec(vec![Param::typed(1_i32), Param::typed("x".to_owned())]),
            cacheable: false,
        };

        let rendered = pretty_query(&built).to_string();

        assert_eq!(
            rendered,
            format!(
                concat!(
                    "SQL:\n",
                    "select $1, $2\n",
                    "\n",
                    "Params (2):\n",
                    "$1: {}\n",
                    "$2: {}\n",
                    "\n",
                    "Cacheable: false"
                ),
                std::any::type_name::<i32>(),
                std::any::type_name::<String>()
            )
        );
        assert_eq!(built.pretty().to_string(), rendered);
    }

    #[test]
    fn pretty_query_formats_empty_params() {
        let built = BuiltQuery {
            sql: "select 1".to_owned(),
            params: Params::new(),
            cacheable: true,
        };

        assert_eq!(
            built.pretty().to_string(),
            concat!(
                "SQL:\n",
                "select 1\n",
                "\n",
                "Params: none\n",
                "\n",
                "Cacheable: true"
            )
        );
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
