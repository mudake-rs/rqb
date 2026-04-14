use rqb_core::{ValidatedQueryExpr, ValidatedSetQuery};

use crate::helpers::write_quoted_ident;
use crate::{BuiltQuery, Result};

use super::{Renderer, SelectProjection};

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
        self.render_query_expr(validated, projection, true)
    }

    pub(super) fn render_query_expr(
        &mut self,
        validated: &ValidatedQueryExpr,
        projection: SelectProjection,
        render_top_limit: bool,
    ) -> Result<()> {
        match validated {
            ValidatedQueryExpr::Select(select) => self.render_subquery_select(select, projection),
            ValidatedQueryExpr::Set(set) => self.render_set_query(set, render_top_limit),
        }
    }

    fn render_set_operand(&mut self, validated: &ValidatedQueryExpr) -> Result<()> {
        match validated {
            ValidatedQueryExpr::Select(select) => self.render_set_select_arm(select),
            ValidatedQueryExpr::Set(set) => self.render_set_query(set, true),
        }
    }

    fn render_set_query(
        &mut self,
        validated: &ValidatedSetQuery,
        render_top_limit: bool,
    ) -> Result<()> {
        self.sql.push('(');
        self.render_set_operand(&validated.left)?;
        self.sql.push(')');
        self.sql.push(' ');
        self.sql.push_str(validated.operator.as_sql());
        self.sql.push(' ');
        self.sql.push('(');
        self.render_set_operand(&validated.right)?;
        self.sql.push(')');
        if render_top_limit {
            self.render_set_order(validated);
            if let Some(limit) = validated.limit {
                self.sql.push_str(" LIMIT ");
                self.sql.push_str(&limit.to_string());
            }
            if let Some(offset) = validated.offset {
                self.sql.push_str(" OFFSET ");
                self.sql.push_str(&offset.to_string());
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
