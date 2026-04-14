use rqb_core::ValidatedDelete;

use crate::{BuiltQuery, Result};

use super::Renderer;

impl Renderer {
    pub(crate) fn render_delete(mut self, validated: &ValidatedDelete) -> Result<BuiltQuery> {
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&validated.dataset.source);
        if !validated.using.is_empty() {
            self.sql.push_str(" USING ");
            for (idx, dataset) in validated.using.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.render_source(&dataset.source)?;
            }
        }
        self.sql.push_str(" WHERE ");
        self.render_expr(&validated.filter)?;
        self.render_returning(&validated.returning)?;
        self.columns = validated
            .returning
            .iter()
            .map(|item| item.column())
            .collect();
        Ok(self.finish())
    }
}
