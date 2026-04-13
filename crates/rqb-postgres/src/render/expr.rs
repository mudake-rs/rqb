use rqb_core::{LogicalOp, ValidatedExpr};

use crate::Result;

use super::{Renderer, SelectProjection};

impl Renderer {
    pub(super) fn render_expr(&mut self, expr: &ValidatedExpr) -> Result<()> {
        match expr {
            ValidatedExpr::Predicate {
                field,
                operator,
                value,
            } => self.render_predicate(field, *operator, value),
            ValidatedExpr::ColumnPredicate {
                left,
                operator,
                right,
            } => self.render_column_predicate(left, *operator, right),
            ValidatedExpr::Subquery {
                field,
                operator,
                query,
            } => {
                self.render_column_name(field);
                self.sql.push(' ');
                self.sql.push_str(operator.as_sql());
                self.sql.push_str(" (");
                self.render_subquery(query, SelectProjection::Value)?;
                self.sql.push(')');
                Ok(())
            }
            ValidatedExpr::Exists { query, negated } => {
                if *negated {
                    self.sql.push_str("NOT ");
                }
                self.sql.push_str("EXISTS (");
                self.render_subquery(query, SelectProjection::Exists)?;
                self.sql.push(')');
                Ok(())
            }
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
            ValidatedExpr::Raw(raw) => {
                self.render_raw(raw);
                Ok(())
            }
        }
    }
}
