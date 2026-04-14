mod aggregate;
mod common;
mod cte;
mod delete;
mod expr;
mod insert;
mod params;
mod predicate;
mod query;
mod select;
mod update;

use rqb_core::{SelectColumn, Value};

use crate::BuiltQuery;

#[derive(Default)]
pub(crate) struct Renderer {
    sql: String,
    params: Vec<Value>,
    columns: Vec<SelectColumn>,
    cacheable: bool,
}

#[derive(Clone, Copy)]
pub(super) enum SelectProjection {
    Value,
    Exists,
}

#[derive(Clone, Copy)]
pub(super) enum LimitPolicy {
    Always,
    ExplicitOnly,
}

impl Renderer {
    pub(crate) fn new() -> Self {
        Self {
            sql: String::with_capacity(256),
            params: Vec::with_capacity(8),
            columns: Vec::new(),
            cacheable: true,
        }
    }

    fn finish(self) -> BuiltQuery {
        BuiltQuery {
            sql: self.sql,
            params: self.params,
            columns: self.columns,
            cacheable: self.cacheable,
        }
    }
}
