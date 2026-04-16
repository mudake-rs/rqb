use crate::typed::{BoolExpr, ValueExpr};

use super::function;

pub fn to_tsvector(document: impl Into<ValueExpr>) -> ValueExpr {
    function("to_tsvector", [document])
}

pub fn to_tsvector_config(
    config: impl Into<ValueExpr>,
    document: impl Into<ValueExpr>,
) -> ValueExpr {
    function(
        "to_tsvector",
        [
            ValueExpr::Cast {
                expr: Box::new(config.into()),
                pg: "regconfig",
            },
            document.into(),
        ],
    )
}

pub fn plainto_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("plainto_tsquery", [query])
}

pub fn websearch_to_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("websearch_to_tsquery", [query])
}

pub fn to_tsquery(query: impl Into<ValueExpr>) -> ValueExpr {
    function("to_tsquery", [query])
}

pub fn ts_match(document: impl Into<ValueExpr>, query: impl Into<ValueExpr>) -> BoolExpr {
    BoolExpr::Infix {
        left: document.into(),
        op: "@@",
        right: query.into(),
        negated: false,
    }
}

pub fn ts_rank(vector: impl Into<ValueExpr>, query: impl Into<ValueExpr>) -> ValueExpr {
    function("ts_rank", [vector.into(), query.into()])
}

pub fn ts_rank_cd(vector: impl Into<ValueExpr>, query: impl Into<ValueExpr>) -> ValueExpr {
    function("ts_rank_cd", [vector.into(), query.into()])
}
