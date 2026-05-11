use crate::ValueExpr;

use super::function;

/// Builds `coalesce(...)`.
pub fn coalesce(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("coalesce", args)
}

/// Builds `nullif(left, right)`.
pub fn nullif(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    function("nullif", [left.into(), right.into()])
}

/// Builds `greatest(...)`.
pub fn greatest(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("greatest", args)
}

/// Builds `least(...)`.
pub fn least(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("least", args)
}

/// Builds the `CURRENT_USER` SQL keyword.
pub fn current_user() -> ValueExpr {
    ValueExpr::Keyword("CURRENT_USER")
}

/// Builds the `SESSION_USER` SQL keyword.
pub fn session_user() -> ValueExpr {
    ValueExpr::Keyword("SESSION_USER")
}

/// Builds the `CURRENT_SCHEMA` SQL keyword.
pub fn current_schema() -> ValueExpr {
    ValueExpr::Keyword("CURRENT_SCHEMA")
}

/// Builds `current_database()`.
pub fn current_database() -> ValueExpr {
    function("current_database", Vec::<ValueExpr>::new())
}

/// Builds `version()`.
pub fn version() -> ValueExpr {
    function("version", Vec::<ValueExpr>::new())
}
