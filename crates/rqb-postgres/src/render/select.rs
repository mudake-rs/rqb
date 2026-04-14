use rqb_core::ValidatedSelect;

use crate::helpers::{needs_count_subquery, write_quoted_ident};
use crate::{BuiltQuery, Result};

use super::{LimitPolicy, Renderer, SelectProjection};

impl Renderer {
    pub(crate) fn render_rows(mut self, validated: &ValidatedSelect) -> Result<BuiltQuery> {
        self.cacheable &= validated.cacheable;
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
        self.cacheable &= validated.cacheable;
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
        } else if validated.distinct {
            self.sql.push_str("DISTINCT ");
        }
    }

    pub(super) fn render_subquery_select(
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

    pub(super) fn render_set_select_arm(&mut self, validated: &ValidatedSelect) -> Result<()> {
        self.cacheable &= validated.cacheable;
        self.render_ctes(validated)?;
        self.sql.push_str("SELECT ");
        self.render_distinct(validated);
        self.render_value_projection(validated, true)?;
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
        if validated.selected_fields.is_empty()
            && validated.aggregates.is_empty()
            && validated.select_items.is_empty()
        {
            self.sql.push('*');
            return Ok(());
        }

        self.render_value_projection(validated, true)
    }

    fn render_subquery_value_projection(&mut self, validated: &ValidatedSelect) -> Result<()> {
        self.render_value_projection(validated, false)
    }

    fn render_value_projection(
        &mut self,
        validated: &ValidatedSelect,
        alias_fields: bool,
    ) -> Result<()> {
        if validated.selected_fields.is_empty()
            && validated.aggregates.is_empty()
            && validated.select_items.is_empty()
        {
            self.sql.push('*');
            return Ok(());
        }

        let mut wrote = false;
        for field in &validated.selected_fields {
            if wrote {
                self.sql.push_str(", ");
            }
            if alias_fields {
                self.render_selected_field(field);
            } else {
                self.render_column_name(field);
            }
            wrote = true;
        }
        for aggregate in &validated.aggregates {
            if wrote {
                self.sql.push_str(", ");
            }
            self.render_aggregate(aggregate)?;
            wrote = true;
        }
        for item in &validated.select_items {
            if wrote {
                self.sql.push_str(", ");
            }
            self.render_selected_expr(item)?;
            wrote = true;
        }
        Ok(())
    }

    fn render_from_and_joins(&mut self, validated: &ValidatedSelect) -> Result<()> {
        self.render_validated_source(&validated.source)?;
        for join in &validated.joins {
            self.sql.push(' ');
            self.sql.push_str(join.kind.as_sql());
            self.sql.push(' ');
            if join.lateral {
                self.sql.push_str("LATERAL ");
            }
            self.render_validated_source(&join.source)?;
            if let Some(on) = &join.on {
                self.sql.push_str(" ON ");
                self.render_expr(on)?;
            }
        }
        Ok(())
    }

    fn render_where(&mut self, validated: &ValidatedSelect) -> Result<()> {
        let Some(expr) = &validated.filter else {
            return Ok(());
        };

        self.sql.push_str(" WHERE ");
        self.render_expr(expr)
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
        let Some(expr) = &validated.having else {
            return Ok(());
        };

        self.sql.push_str(" HAVING ");
        self.render_expr(expr)
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
            self.render_sort(sort);
        }
    }

    fn render_limit_offset(&mut self, validated: &ValidatedSelect, policy: LimitPolicy) {
        if matches!(policy, LimitPolicy::Always) || validated.limit_explicit {
            self.sql.push_str(" LIMIT ");
            self.sql.push_str(&validated.limit.to_string());
        }
        if matches!(policy, LimitPolicy::Always) || validated.offset_explicit {
            self.sql.push_str(" OFFSET ");
            self.sql.push_str(&validated.offset.to_string());
        }
    }

    fn render_row_lock(&mut self, validated: &ValidatedSelect) {
        let Some(lock) = validated.lock else {
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
