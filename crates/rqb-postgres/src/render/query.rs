use rqb_core::{ValidatedQueryExpr, ValidatedSetQuery};

use crate::helpers::write_quoted_ident;
use crate::{BuiltQuery, Result};

use super::{Renderer, SelectProjection, SetRenderMode};

impl Renderer {
    pub(crate) fn render_query_rows(
        mut self,
        validated: &ValidatedQueryExpr,
    ) -> Result<BuiltQuery> {
        self.columns = validated.columns().to_vec();
        self.cacheable &= validated.cacheable();
        self.render_query_expr(validated, SelectProjection::Value, true)?;
        Ok(self.finish())
    }

    pub(crate) fn render_query_count(
        mut self,
        validated: &ValidatedQueryExpr,
    ) -> Result<BuiltQuery> {
        self.cacheable &= validated.cacheable();
        self.sql.push_str("SELECT count(*) FROM (");
        self.render_query_expr(validated, SelectProjection::Value, false)?;
        self.sql.push_str(") AS ");
        write_quoted_ident(&mut self.sql, "rqb_count");
        Ok(self.finish())
    }

    pub(super) fn render_subquery(
        &mut self,
        validated: &ValidatedQueryExpr,
        projection: SelectProjection,
    ) -> Result<()> {
        self.cacheable &= validated.cacheable();
        match validated {
            ValidatedQueryExpr::Select(select) => self.render_subquery_select(select, projection),
            ValidatedQueryExpr::Set(set) => self.render_set_query(set, true, SetRenderMode::Source),
        }
    }

    pub(super) fn render_query_expr(
        &mut self,
        validated: &ValidatedQueryExpr,
        projection: SelectProjection,
        render_top_limit: bool,
    ) -> Result<()> {
        match validated {
            ValidatedQueryExpr::Select(select) => self.render_subquery_select(select, projection),
            ValidatedQueryExpr::Set(set) => {
                self.render_set_query(set, render_top_limit, SetRenderMode::QueryResult)
            }
        }
    }

    pub(super) fn render_query_source(&mut self, validated: &ValidatedQueryExpr) -> Result<()> {
        self.cacheable &= validated.cacheable();
        match validated {
            ValidatedQueryExpr::Select(select) => {
                self.render_subquery_select(select, SelectProjection::Value)
            }
            ValidatedQueryExpr::Set(set) => self.render_set_query(set, true, SetRenderMode::Source),
        }
    }

    fn render_set_operand(
        &mut self,
        validated: &ValidatedQueryExpr,
        mode: SetRenderMode,
    ) -> Result<()> {
        match validated {
            ValidatedQueryExpr::Select(select) => match mode {
                SetRenderMode::QueryResult => self.render_set_select_arm(select),
                SetRenderMode::Source => {
                    self.render_subquery_select(select, SelectProjection::Value)
                }
            },
            ValidatedQueryExpr::Set(set) => self.render_set_query(set, true, mode),
        }
    }

    fn render_set_query(
        &mut self,
        validated: &ValidatedSetQuery,
        render_top_limit: bool,
        mode: SetRenderMode,
    ) -> Result<()> {
        self.sql.push('(');
        self.render_set_operand(&validated.left, mode)?;
        self.sql.push(')');
        self.sql.push(' ');
        self.sql.push_str(validated.operator.as_sql());
        self.sql.push(' ');
        self.sql.push('(');
        self.render_set_operand(&validated.right, mode)?;
        self.sql.push(')');
        if render_top_limit {
            self.render_set_order(validated);
            if let Some(limit) = validated.limit {
                self.sql.push_str(" LIMIT ");
                let mut buffer = itoa::Buffer::new();
                self.sql.push_str(buffer.format(limit));
            }
            if let Some(offset) = validated.offset {
                self.sql.push_str(" OFFSET ");
                let mut buffer = itoa::Buffer::new();
                self.sql.push_str(buffer.format(offset));
            }
        }
        Ok(())
    }

    fn render_set_order(&mut self, validated: &ValidatedSetQuery) {
        if validated.sort.is_empty() {
            return;
        }

        self.sql.push_str(" ORDER BY ");
        for (idx, sort) in validated.sort.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, &sort.alias);
            self.sql.push(' ');
            self.sql.push_str(sort.dir.as_str());
            if let Some(nulls) = sort.nulls {
                self.sql.push(' ');
                self.sql.push_str(nulls.as_str());
            }
        }
    }
}
