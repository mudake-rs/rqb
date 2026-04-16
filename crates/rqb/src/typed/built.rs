use sqlx::postgres::PgArguments;

use crate::Result;
use crate::typed::Params;

#[derive(Clone, Debug)]
#[must_use]
pub struct BuiltQuery {
    pub sql: String,
    pub params: Params,
    pub cacheable: bool,
}

impl BuiltQuery {
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
}
