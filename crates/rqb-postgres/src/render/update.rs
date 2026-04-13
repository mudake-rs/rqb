use rqb_core::{SelectColumn, ValidatedUpdate};

use crate::helpers::write_quoted_ident;
use crate::{BuiltQuery, Result};

use super::Renderer;

impl Renderer {
    pub(crate) fn render_update(mut self, validated: &ValidatedUpdate) -> Result<BuiltQuery> {
        self.cacheable = false;
        self.sql.push_str("UPDATE ");
        self.render_write_target(&validated.query.dataset.source);
        self.sql.push_str(" SET ");
        for (idx, assignment) in validated.assignments.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, &assignment.field.db_name);
            self.sql.push_str(" = ");
            self.render_write_value(&assignment.value, assignment.field.ty)?;
        }

        if let Some(expr) = &validated.filter {
            self.sql.push_str(" WHERE ");
            self.render_expr(expr)?;
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
}
