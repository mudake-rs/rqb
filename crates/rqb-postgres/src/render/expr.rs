use rqb_core::{LogicalOp, ValidatedExpr};

use crate::Result;

use super::Renderer;

impl Renderer {
    pub(super) fn render_expr(&mut self, expr: &ValidatedExpr) -> Result<()> {
        match expr {
            ValidatedExpr::Predicate(predicate) => self.render_predicate(predicate),
            ValidatedExpr::Logical {
                logical,
                predicates,
            } => match logical {
                LogicalOp::And | LogicalOp::Or => {
                    let sep = if *logical == LogicalOp::And {
                        " AND "
                    } else {
                        " OR "
                    };
                    self.sql.push('(');
                    for (idx, predicate) in predicates.iter().enumerate() {
                        if idx > 0 {
                            self.sql.push_str(sep);
                        }
                        self.render_expr(predicate)?;
                    }
                    self.sql.push(')');
                    Ok(())
                }
                LogicalOp::Not => {
                    self.sql.push_str("NOT (");
                    self.render_expr(&predicates[0])?;
                    self.sql.push(')');
                    Ok(())
                }
            },
        }
    }
}
