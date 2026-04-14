use std::fmt;

use rqb_core::SelectColumn;

#[cfg(feature = "runtime-tokio-postgres")]
use crate::PgParams;
use crate::bind::BindParam;

/// A single rendered Postgres statement with collected bind parameters.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<BindParam>,
    pub columns: Vec<SelectColumn>,
    pub cacheable: bool,
}

impl BuiltQuery {
    pub fn debug_sql(&self) -> DebugSql<'_> {
        DebugSql { query: self }
    }
}

#[must_use]
pub struct DebugSql<'a> {
    query: &'a BuiltQuery,
}

impl fmt::Display for DebugSql<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.query.sql)?;
        write!(f, "-- params: {:?}", self.query.params)
    }
}

#[cfg(feature = "runtime-tokio-postgres")]
impl BuiltQuery {
    pub fn params(&self) -> PgParams {
        PgParams::from_binds(&self.params)
    }
}

/// Rendered SELECT statements for page-style execution.
///
/// rqb renders both the rows query and matching count query for a select,
/// because the same builder powers list endpoints and JSON search APIs.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct BuiltSelect {
    pub rows: BuiltQuery,
    pub count: BuiltQuery,
}

impl BuiltSelect {
    pub fn debug_sql(&self) -> DebugSelectSql<'_> {
        DebugSelectSql { select: self }
    }
}

#[must_use]
pub struct DebugSelectSql<'a> {
    select: &'a BuiltSelect,
}

impl fmt::Display for DebugSelectSql<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "-- rows")?;
        writeln!(f, "{}", self.select.rows.debug_sql())?;
        writeln!(f)?;
        writeln!(f, "-- count")?;
        write!(f, "{}", self.select.count.debug_sql())
    }
}
