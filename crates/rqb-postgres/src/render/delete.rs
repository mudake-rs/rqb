use rqb_core::{SelectColumn, SelectQuery, ValidatedDelete, ValidatedSelect};

use crate::{BuiltQuery, Result};

use super::Renderer;

impl Renderer {
    pub(crate) fn render_delete(mut self, validated: &ValidatedDelete) -> Result<BuiltQuery> {
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&validated.query.dataset.source);
        if let Some(expr) = &validated.query.filter {
            self.sql.push_str(" WHERE ");
            let select = ValidatedSelect::new(SelectQuery::new(validated.query.dataset.clone()))?;
            self.render_expr(&select, expr)?;
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
