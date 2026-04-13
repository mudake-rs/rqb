use rqb_core::{
    SelectColumn, ValidatedConflictAction, ValidatedConflictClause, ValidatedConflictTarget,
    ValidatedInsert,
};

use crate::helpers::write_quoted_ident;
use crate::{BuiltQuery, Result};

use super::Renderer;

impl Renderer {
    pub(crate) fn render_insert(mut self, validated: &ValidatedInsert) -> Result<BuiltQuery> {
        if validated.rows.len() > 1 {
            self.cacheable = false;
        }
        self.sql.push_str("INSERT INTO ");
        self.render_write_target(&validated.query.dataset.source);

        let target_fields = if validated.from_select.is_some() {
            validated.from_select_targets.clone()
        } else {
            validated
                .rows
                .first()
                .map(|row| {
                    row.iter()
                        .map(|assignment| assignment.field.clone())
                        .collect()
                })
                .unwrap_or_default()
        };
        self.render_insert_columns(&target_fields);

        if let Some(select) = &validated.from_select {
            self.sql.push(' ');
            self.render_subquery_select(select, super::SelectProjection::Value)?;
        } else {
            self.sql.push_str(" VALUES ");
            for (row_idx, row) in validated.rows.iter().enumerate() {
                if row_idx > 0 {
                    self.sql.push_str(", ");
                }
                self.sql.push('(');
                for (value_idx, assignment) in row.iter().enumerate() {
                    if value_idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_write_value(&assignment.value, assignment.field.ty)?;
                }
                self.sql.push(')');
            }
        }

        if let Some(conflict) = &validated.conflict {
            self.render_conflict(conflict)?;
        }
        self.render_returning(&validated.returning);
        self.columns = validated
            .returning
            .iter()
            .cloned()
            .map(SelectColumn::Field)
            .collect();
        Ok(self.finish())
    }

    fn render_conflict(&mut self, conflict: &ValidatedConflictClause) -> Result<()> {
        self.sql.push_str(" ON CONFLICT ");
        match &conflict.target {
            ValidatedConflictTarget::Columns(fields) => {
                self.sql.push('(');
                for (idx, field) in fields.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    write_quoted_ident(&mut self.sql, &field.db_name);
                }
                self.sql.push(')');
            }
            ValidatedConflictTarget::Constraint(constraint) => {
                self.sql.push_str("ON CONSTRAINT ");
                write_quoted_ident(&mut self.sql, constraint);
            }
        }

        match &conflict.action {
            ValidatedConflictAction::DoNothing => self.sql.push_str(" DO NOTHING"),
            ValidatedConflictAction::DoUpdate { fields, filter } => {
                self.sql.push_str(" DO UPDATE SET ");
                for (idx, field) in fields.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    write_quoted_ident(&mut self.sql, &field.db_name);
                    self.sql.push_str(" = EXCLUDED.");
                    write_quoted_ident(&mut self.sql, &field.db_name);
                }
                if let Some(filter) = filter {
                    self.sql.push_str(" WHERE ");
                    self.render_expr(filter)?;
                }
            }
        }
        Ok(())
    }
}
