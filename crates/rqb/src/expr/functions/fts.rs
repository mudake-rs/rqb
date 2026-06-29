use crate::{BoolExpr, ValueExpr};

use super::function;

/// Builds `to_tsvector(document)`.
pub fn to_tsvector(document: impl Into<ValueExpr>) -> ValueExpr {
    function("to_tsvector", [document])
}

/// Builds `to_tsvector(config::regconfig, document)`.
pub fn to_tsvector_config(
    config: impl Into<ValueExpr>,
    document: impl Into<ValueExpr>,
) -> ValueExpr {
    function(
        "to_tsvector",
        [
            ValueExpr::cast_expr(config.into(), "regconfig"),
            document.into(),
        ],
    )
}

/// Builds `plainto_tsquery(query)`.
pub fn plainto_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("plainto_tsquery", [query])
}

/// Builds `phraseto_tsquery(query)`.
pub fn phraseto_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("phraseto_tsquery", [query])
}

/// Builds `phraseto_tsquery(config::regconfig, query)`.
pub fn phraseto_tsquery_config(
    config: impl Into<ValueExpr>,
    query: impl Into<ValueExpr>,
) -> ValueExpr {
    function(
        "phraseto_tsquery",
        [
            ValueExpr::cast_expr(config.into(), "regconfig"),
            query.into(),
        ],
    )
}

/// Builds `websearch_to_tsquery(query)`.
pub fn websearch_to_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("websearch_to_tsquery", [query])
}

/// Builds `to_tsquery(query)`.
pub fn to_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("to_tsquery", [query])
}

/// Builds a full-text `@@` match predicate.
pub fn ts_match(document: impl Into<ValueExpr>, query: impl Into<ValueExpr>) -> BoolExpr {
    BoolExpr::infix(document.into(), "@@", query.into(), false)
}

/// Builds `ts_rank(vector, query)`.
pub fn ts_rank(vector: impl Into<ValueExpr>, query: impl Into<ValueExpr>) -> ValueExpr {
    function("ts_rank", [vector.into(), query.into()])
}

/// Builds `ts_rank_cd(vector, query)`.
pub fn ts_rank_cd(vector: impl Into<ValueExpr>, query: impl Into<ValueExpr>) -> ValueExpr {
    function("ts_rank_cd", [vector.into(), query.into()])
}

/// Builds `ts_headline(...)`.
pub fn ts_headline(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("ts_headline", args)
}
