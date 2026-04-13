use rqb_core::{SelectQuery, ValidatedSelect};

use crate::helpers::{needs_count_subquery, write_quoted_ident};
use crate::{BuiltQuery, Result};

use super::{LimitPolicy, Renderer, SelectProjection};

impl Renderer {
    pub(crate) fn render_rows(mut self, validated: &ValidatedSelect) -> Result<BuiltQuery> {
        self.cacheable &= validated.query.cacheable;
        self.render_ctes(validated)?;
        self.sql.push_str("SELECT ");
        self.render_distinct(validated);
        self.render_selection(validated)?;
        self.sql.push_str(" FROM ");
        self.render_from_and_joins(validated)?;
        self.render_where(validated)?;
        self.render_group_by(validated);
        self.render_having(validated)?;
        self.render_order(validated);
        self.render_limit_offset(validated, LimitPolicy::Always);
        self.render_row_lock(validated);
        Ok(self.finish())
    }

    pub(crate) fn render_count(mut self, validated: &ValidatedSelect) -> Result<BuiltQuery> {
        self.cacheable &= validated.query.cacheable;
        self.render_ctes(validated)?;
        if needs_count_subquery(validated) {
            self.sql.push_str("SELECT count(*) FROM (SELECT ");
            self.render_distinct(validated);
            self.render_selection(validated)?;
            self.sql.push_str(" FROM ");
            self.render_from_and_joins(validated)?;
            self.render_where(validated)?;
            self.render_group_by(validated);
            self.render_having(validated)?;
            self.sql.push_str(") AS ");
            write_quoted_ident(&mut self.sql, "rqb_count");
        } else {
            self.sql.push_str("SELECT count(*) FROM ");
            self.render_from_and_joins(validated)?;
            self.render_where(validated)?;
        }
        Ok(self.finish())
    }

    fn render_distinct(&mut self, validated: &ValidatedSelect) {
        if !validated.distinct_on.is_empty() {
            self.sql.push_str("DISTINCT ON (");
            for (idx, field) in validated.distinct_on.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.render_column_name(field);
            }
            self.sql.push_str(") ");
        } else if validated.query.distinct {
            self.sql.push_str("DISTINCT ");
        }
    }

    pub(super) fn render_subquery(
        &mut self,
        outer: &ValidatedSelect,
        query: &SelectQuery,
        projection: SelectProjection,
    ) -> Result<()> {
        let mut outer_datasets = self.outer_datasets.clone();
        outer_datasets.extend(outer.query.scope_datasets());
        let validated = ValidatedSelect::new_with_outer_datasets(query.clone(), &outer_datasets)?;
        self.cacheable &= validated.query.cacheable;

        let previous = std::mem::replace(&mut self.outer_datasets, outer_datasets);
        let result = self.render_subquery_select(&validated, projection);
        self.outer_datasets = previous;
        result
    }

    fn render_subquery_select(
        &mut self,
        validated: &ValidatedSelect,
        projection: SelectProjection,
    ) -> Result<()> {
        self.render_ctes(validated)?;
        self.sql.push_str("SELECT ");
        match projection {
            SelectProjection::Value => {
                self.render_distinct(validated);
                self.render_subquery_value_projection(validated)?;
            }
            SelectProjection::Exists => self.sql.push('1'),
        }
        self.sql.push_str(" FROM ");
        self.render_from_and_joins(validated)?;
        self.render_where(validated)?;
        self.render_group_by(validated);
        self.render_having(validated)?;
        self.render_order(validated);
        self.render_limit_offset(validated, LimitPolicy::ExplicitOnly);
        self.render_row_lock(validated);
        Ok(())
    }

    fn render_selection(&mut self, validated: &ValidatedSelect) -> Result<()> {
        self.columns.clone_from(&validated.columns);
        if validated.selected_fields.is_empty() && validated.aggregates.is_empty() {
            self.sql.push('*');
            return Ok(());
        }

        let mut wrote = false;
        for field in &validated.selected_fields {
            if wrote {
                self.sql.push_str(", ");
            }
            self.render_selected_field(field);
            wrote = true;
        }
        for aggregate in &validated.aggregates {
            if wrote {
                self.sql.push_str(", ");
            }
            self.render_aggregate(validated, aggregate)?;
            wrote = true;
        }
        Ok(())
    }

    fn render_subquery_value_projection(&mut self, validated: &ValidatedSelect) -> Result<()> {
        if validated.selected_fields.is_empty() && validated.aggregates.is_empty() {
            self.sql.push('*');
            return Ok(());
        }

        let mut wrote = false;
        for field in &validated.selected_fields {
            if wrote {
                self.sql.push_str(", ");
            }
            self.render_column_name(field);
            wrote = true;
        }
        for aggregate in &validated.aggregates {
            if wrote {
                self.sql.push_str(", ");
            }
            self.render_aggregate(validated, aggregate)?;
            wrote = true;
        }
        Ok(())
    }

    fn render_from_and_joins(&mut self, validated: &ValidatedSelect) -> Result<()> {
        self.render_source(&validated.query.dataset.source);
        for join in &validated.query.joins {
            self.sql.push(' ');
            self.sql.push_str(join.kind.as_sql());
            self.sql.push(' ');
            self.render_source(&join.dataset.source);
            if let Some(on) = &join.on {
                self.sql.push_str(" ON ");
                self.render_expr(validated, on)?;
            }
        }
        Ok(())
    }

    fn render_where(&mut self, validated: &ValidatedSelect) -> Result<()> {
        let Some(expr) = &validated.query.request.query else {
            return Ok(());
        };

        self.sql.push_str(" WHERE ");
        self.render_expr(validated, expr)
    }

    fn render_group_by(&mut self, validated: &ValidatedSelect) {
        if validated.group_by.is_empty() {
            return;
        }

        self.sql.push_str(" GROUP BY ");
        for (idx, field) in validated.group_by.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_column_name(field);
        }
    }

    fn render_having(&mut self, validated: &ValidatedSelect) -> Result<()> {
        let Some(expr) = &validated.query.having else {
            return Ok(());
        };

        self.sql.push_str(" HAVING ");
        self.render_expr(validated, expr)
    }

    fn render_order(&mut self, validated: &ValidatedSelect) {
        if validated.sort.is_empty() {
            return;
        }

        self.sql.push_str(" ORDER BY ");
        for (idx, sort) in validated.sort.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_column_name(&sort.field);
            self.sql.push(' ');
            self.sql.push_str(sort.dir.as_str());
            if let Some(nulls) = sort.nulls {
                self.sql.push(' ');
                self.sql.push_str(nulls.as_str());
            }
        }
    }

    fn render_limit_offset(&mut self, validated: &ValidatedSelect, policy: LimitPolicy) {
        if matches!(policy, LimitPolicy::Always) || validated.query.request.limit.is_some() {
            self.sql.push_str(" LIMIT ");
            self.sql.push_str(&validated.limit.to_string());
        }
        if matches!(policy, LimitPolicy::Always) || validated.query.request.offset.is_some() {
            self.sql.push_str(" OFFSET ");
            self.sql.push_str(&validated.offset.to_string());
        }
    }

    fn render_row_lock(&mut self, validated: &ValidatedSelect) {
        let Some(lock) = validated.query.lock else {
            return;
        };

        self.sql.push(' ');
        self.sql.push_str(lock.mode.as_sql());
        if let Some(wait) = lock.wait.as_sql() {
            self.sql.push(' ');
            self.sql.push_str(wait);
        }
    }
}
