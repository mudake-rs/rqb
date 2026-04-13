use rqb_core::{SelectColumn, ValidatedDelete};

use crate::{BuiltQuery, Result};

use super::Renderer;

impl Renderer {
    pub(crate) fn render_delete(mut self, validated: &ValidatedDelete) -> Result<BuiltQuery> {
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&validated.dataset.source);
        self.sql.push_str(" WHERE ");
        self.render_expr(&validated.filter)?;
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
