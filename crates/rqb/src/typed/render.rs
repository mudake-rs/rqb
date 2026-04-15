use crate::Result;
use crate::typed::ident::{write_quoted_ident, write_quoted_qualified};
use crate::typed::{
    Assignment, BoolExpr, BuiltQuery, ConflictAction, ConflictClause, ConflictTarget, Cte, Delete,
    Insert, Param, Params, RawStmt, Select, SelectItem, SetQuery, Source, Stmt, ValueExpr, ValueOp,
};

mod expr;
mod raw;
mod source;
mod stmt;
mod write;

#[derive(Default)]
struct Renderer {
    sql: String,
    params: Vec<Param>,
    cacheable: bool,
}

impl Renderer {
    fn new() -> Self {
        Self {
            sql: String::with_capacity(256),
            params: Vec::with_capacity(8),
            cacheable: true,
        }
    }

    fn finish(self) -> BuiltQuery {
        BuiltQuery {
            sql: self.sql,
            params: Params::from_vec(self.params),
            cacheable: self.cacheable,
        }
    }

    fn build_with(render: impl FnOnce(&mut Self) -> Result<()>) -> Result<BuiltQuery> {
        let mut renderer = Self::new();
        render(&mut renderer)?;
        Ok(renderer.finish())
    }
}

impl Stmt {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_stmt(self))
    }
}

impl Select {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_select(self))
    }
}

impl SetQuery {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_set(self))
    }
}

impl Insert {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_insert(self))
    }
}

impl crate::typed::Update {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_update(self))
    }
}

impl Delete {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_delete(self))
    }
}

impl RawStmt {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_raw_stmt(self))
    }
}

#[cfg(test)]
mod tests;
