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
