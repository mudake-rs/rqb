use super::*;

impl Renderer {
    pub(super) fn render_bool(&mut self, expr: &BoolExpr) -> Result<()> {
        match expr {
            BoolExpr::Constant(value) => {
                self.sql.push_str(if *value { "TRUE" } else { "FALSE" });
                Ok(())
            }
            BoolExpr::Compare { left, op, right } => {
                self.render_value(left)?;
                self.sql.push(' ');
                self.sql.push_str(op.as_sql());
                self.sql.push(' ');
                self.render_value(right)
            }
            BoolExpr::IsNull { expr, negated } => {
                self.render_value(expr)?;
                self.sql
                    .push_str(if *negated { " IS NOT NULL" } else { " IS NULL" });
                Ok(())
            }
            BoolExpr::IsBoolean {
                expr,
                test,
                negated,
            } => {
                self.render_value(expr)?;
                self.sql
                    .push_str(if *negated { " IS NOT " } else { " IS " });
                self.sql.push_str(test.as_sql());
                Ok(())
            }
            BoolExpr::InList {
                expr,
                values,
                negated,
            } => {
                self.render_value(expr)?;
                self.sql
                    .push_str(if *negated { " NOT IN (" } else { " IN (" });
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_value(value)?;
                }
                self.sql.push(')');
                Ok(())
            }
            BoolExpr::InSubquery {
                expr,
                query,
                negated,
            } => {
                self.render_value(expr)?;
                self.sql
                    .push_str(if *negated { " NOT IN (" } else { " IN (" });
                self.render_stmt(query)?;
                self.sql.push(')');
                Ok(())
            }
            BoolExpr::Between {
                expr,
                low,
                high,
                negated,
            } => {
                self.render_value(expr)?;
                self.sql.push_str(if *negated {
                    " NOT BETWEEN "
                } else {
                    " BETWEEN "
                });
                self.render_value(low)?;
                self.sql.push_str(" AND ");
                self.render_value(high)
            }
            BoolExpr::Like {
                expr,
                pattern,
                case_insensitive,
                negated,
                escape,
            } => {
                self.render_value(expr)?;
                let op = match (*case_insensitive, *negated) {
                    (false, false) => " LIKE ",
                    (false, true) => " NOT LIKE ",
                    (true, false) => " ILIKE ",
                    (true, true) => " NOT ILIKE ",
                };
                self.sql.push_str(op);
                self.render_value(pattern)?;
                if *escape {
                    self.sql.push_str(" ESCAPE '\\'");
                }
                Ok(())
            }
            BoolExpr::SimilarTo {
                expr,
                pattern,
                negated,
            } => {
                self.render_value(expr)?;
                self.sql.push_str(if *negated {
                    " NOT SIMILAR TO "
                } else {
                    " SIMILAR TO "
                });
                self.render_value(pattern)
            }
            BoolExpr::Regex {
                expr,
                pattern,
                case_insensitive,
                negated,
            } => {
                self.render_value(expr)?;
                let op = match (case_insensitive, negated) {
                    (false, false) => " ~ ",
                    (false, true) => " !~ ",
                    (true, false) => " ~* ",
                    (true, true) => " !~* ",
                };
                self.sql.push_str(op);
                self.render_value(pattern)
            }
            BoolExpr::Infix {
                left,
                op,
                right,
                negated,
            } => {
                if *negated {
                    self.sql.push_str("NOT (");
                }
                self.render_value(left)?;
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
                self.render_value(right)?;
                if *negated {
                    self.sql.push(')');
                }
                Ok(())
            }
            BoolExpr::Any {
                value,
                array,
                negated,
            } => {
                if *negated {
                    self.sql.push_str("NOT (");
                }
                self.render_value(value)?;
                self.sql.push_str(" = ANY(");
                self.render_value(array)?;
                self.sql.push(')');
                if *negated {
                    self.sql.push(')');
                }
                Ok(())
            }
            BoolExpr::ArrayIsEmpty { expr, negated } => {
                self.sql.push_str("cardinality(");
                self.render_value(expr)?;
                self.sql.push_str(if *negated { ") > 0" } else { ") = 0" });
                Ok(())
            }
            BoolExpr::And(exprs) => self.render_bool_list("AND", exprs),
            BoolExpr::Or(exprs) => self.render_bool_list("OR", exprs),
            BoolExpr::Not(expr) => {
                self.sql.push_str("NOT (");
                self.render_bool(expr)?;
                self.sql.push(')');
                Ok(())
            }
            BoolExpr::Exists(stmt) => {
                self.sql.push_str("EXISTS (");
                self.render_stmt(stmt)?;
                self.sql.push(')');
                Ok(())
            }
            BoolExpr::Raw { sql, params } => self.render_raw(sql, params),
        }
    }

    pub(super) fn render_bool_list(&mut self, op: &str, exprs: &[BoolExpr]) -> Result<()> {
        self.sql.push('(');
        for (idx, expr) in exprs.iter().enumerate() {
            if idx > 0 {
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
            }
            self.render_bool(expr)?;
        }
        self.sql.push(')');
        Ok(())
    }
}
