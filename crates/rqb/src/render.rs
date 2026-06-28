use crate::Result;
use crate::ident::{write_quoted_ident, write_quoted_qualified};
use crate::{
    Assignment, BoolExpr, BuiltQuery, ConflictAction, ConflictClause, ConflictTarget, Cte, Delete,
    FetchClause, FrameBound, GroupByItem, Insert, Merge, MergeAction, MergeWhen, Param, Params,
    RawStmt, Select, SelectItem, SetQuery, Source, Stmt, ValueExpr, WindowFrame,
};

mod bool;
mod raw;
mod source;
mod stmt;
mod value;
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
            params: Vec::new(),
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
    /// Validates and renders this statement into parameterized SQL.
    ///
    /// Use this when you need a reusable [`BuiltQuery`] or want to inspect SQL
    /// before execution. The convenience `fetch_*` and `execute` methods build
    /// on each call.
    pub fn build(&self) -> Result<BuiltQuery> {
        if let Self::Raw(raw) = self {
            return raw.build();
        }
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_stmt(self))
    }
}

impl Select {
    /// Validates and renders this select into parameterized SQL.
    ///
    /// Default projection is expanded during render when no explicit projection
    /// items were added.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_select(self))
    }
}

impl SetQuery {
    /// Validates and renders this set query into parameterized SQL.
    ///
    /// Validation checks both sides before rendering the set operator, ordering,
    /// and row-limit clauses.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_set(self))
    }
}

impl Insert {
    /// Validates and renders this insert into parameterized SQL.
    ///
    /// Validation rejects empty insert shapes and write targets that are not
    /// table/view sources before SQL is produced.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_insert(self))
    }
}

impl crate::Update {
    /// Validates and renders this update into parameterized SQL.
    ///
    /// Validation rejects empty `SET` assignments and non-table/view targets
    /// before SQL is produced.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_update(self))
    }
}

impl Delete {
    /// Validates and renders this delete into parameterized SQL.
    ///
    /// Deletes require a `WHERE` predicate; validation returns
    /// [`Error::DeleteWithoutFilter`](crate::Error::DeleteWithoutFilter) before
    /// any SQL is rendered.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_delete(self))
    }
}

impl Merge {
    /// Validates and renders this merge into parameterized SQL.
    ///
    /// Branch/action legality is checked at build time, including which actions
    /// are valid for each `WHEN` branch.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        Renderer::build_with(|renderer| renderer.render_merge(self))
    }
}

impl RawStmt {
    /// Validates and renders this raw statement into parameterized SQL.
    ///
    /// `?` placeholders are converted into Postgres `$N` binds. `??` renders a
    /// literal question mark, and placeholder-like text inside SQL strings,
    /// quoted identifiers, dollar quotes, and comments is ignored.
    pub fn build(&self) -> Result<BuiltQuery> {
        self.validate()?;
        if self.params.is_empty() && !self.sql.as_bytes().contains(&b'?') {
            return Ok(BuiltQuery {
                sql: self.sql.clone(),
                params: Params::new(),
                cacheable: false,
            });
        }
        Renderer::build_with(|renderer| renderer.render_raw_stmt(self))
    }
}

#[cfg(test)]
mod tests;
