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
}

impl Stmt {
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        let mut renderer = Renderer::new();
        renderer.render_stmt(self)?;
        Ok(renderer.finish())
    }
}

impl Select {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Select(Box::new(self.clone())).build()
    }
}

impl SetQuery {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Set(Box::new(self.clone())).build()
    }
}

impl Insert {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Insert(Box::new(self.clone())).build()
    }
}

impl crate::typed::Update {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Update(Box::new(self.clone())).build()
    }
}

impl Delete {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Delete(Box::new(self.clone())).build()
    }
}

impl RawStmt {
    pub fn build(&self) -> Result<BuiltQuery> {
        Stmt::Raw(self.clone()).build()
    }
}

#[cfg(test)]
mod tests;
