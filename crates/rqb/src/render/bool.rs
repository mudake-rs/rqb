use super::*;

impl Renderer {
    pub(super) fn render_bool(&mut self, expr: &BoolExpr) {
        match expr {
            BoolExpr::Constant(value) => {
                self.sql.push_str(if *value { "TRUE" } else { "FALSE" });
            }
            BoolExpr::Compare { left, op, right } => {
                self.render_operand(left);
                self.sql.push(' ');
                self.sql.push_str(op.as_sql());
                self.sql.push(' ');
                self.render_operand(right)
            }
            BoolExpr::IsNull { expr, negated } => {
                self.render_operand(expr);
                self.sql
                    .push_str(if *negated { " IS NOT NULL" } else { " IS NULL" });
            }
            BoolExpr::IsBoolean {
                expr,
                test,
                negated,
            } => {
                self.render_operand(expr);
                self.sql
                    .push_str(if *negated { " IS NOT " } else { " IS " });
                self.sql.push_str(test.as_sql());
            }
            BoolExpr::InList {
                expr,
                values,
                negated,
            } => {
                self.render_operand(expr);
                self.sql
                    .push_str(if *negated { " NOT IN (" } else { " IN (" });
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_operand(value);
                }
                self.sql.push(')');
            }
            BoolExpr::InSubquery {
                expr,
                query,
                negated,
            } => {
                self.render_operand(expr);
                self.sql
                    .push_str(if *negated { " NOT IN (" } else { " IN (" });
                self.render_stmt(query);
                self.sql.push(')');
            }
            BoolExpr::Between {
                expr,
                low,
                high,
                negated,
            } => {
                self.render_operand(expr);
                self.sql.push_str(if *negated {
                    " NOT BETWEEN "
                } else {
                    " BETWEEN "
                });
                self.render_operand(low);
                self.sql.push_str(" AND ");
                self.render_operand(high)
            }
            BoolExpr::Like {
                expr,
                pattern,
                case_insensitive,
                negated,
                escape,
            } => {
                self.render_operand(expr);
                let op = match (*case_insensitive, *negated) {
                    (false, false) => " LIKE ",
                    (false, true) => " NOT LIKE ",
                    (true, false) => " ILIKE ",
                    (true, true) => " NOT ILIKE ",
                };
                self.sql.push_str(op);
                self.render_operand(pattern);
                if *escape {
                    self.sql.push_str(" ESCAPE '\\'");
                }
            }
            BoolExpr::SimilarTo {
                expr,
                pattern,
                negated,
            } => {
                self.render_operand(expr);
                self.sql.push_str(if *negated {
                    " NOT SIMILAR TO "
                } else {
                    " SIMILAR TO "
                });
                self.render_operand(pattern)
            }
            BoolExpr::Regex {
                expr,
                pattern,
                case_insensitive,
                negated,
            } => {
                self.render_operand(expr);
                let op = match (case_insensitive, negated) {
                    (false, false) => " ~ ",
                    (false, true) => " !~ ",
                    (true, false) => " ~* ",
                    (true, true) => " !~* ",
                };
                self.sql.push_str(op);
                self.render_operand(pattern)
            }
            BoolExpr::Infix {
                left,
                op,
                right,
                negated,
                ..
            } => {
                if *negated {
                    self.sql.push_str("NOT (");
                }
                self.render_operand(left);
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
                self.render_operand(right);
                if *negated {
                    self.sql.push(')');
                }
            }
            BoolExpr::Any {
                value,
                array,
                negated,
            } => {
                if *negated {
                    self.sql.push_str("NOT (");
                }
                self.render_operand(value);
                self.sql.push_str(" = ANY(");
                self.render_operand(array);
                self.sql.push(')');
                if *negated {
                    self.sql.push(')');
                }
            }
            BoolExpr::ArrayIsEmpty { expr, negated } => {
                self.sql.push_str("cardinality(");
                self.render_operand(expr);
                self.sql.push_str(if *negated { ") > 0" } else { ") = 0" });
            }
            BoolExpr::And(exprs) => self.render_bool_list("AND", exprs),
            BoolExpr::Or(exprs) => self.render_bool_list("OR", exprs),
            BoolExpr::Not(expr) => {
                self.sql.push_str("NOT (");
                self.render_bool(expr);
                self.sql.push(')');
            }
            BoolExpr::Exists(stmt) => {
                self.sql.push_str("EXISTS (");
                self.render_stmt(stmt);
                self.sql.push(')');
            }
            BoolExpr::Raw { sql, params } => self.render_raw(sql, params),
        }
    }

    pub(super) fn render_bool_list(&mut self, op: &str, exprs: &[BoolExpr]) {
        self.sql.push('(');
        for (idx, expr) in exprs.iter().enumerate() {
            if idx > 0 {
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
            }
            let raw = matches!(expr, BoolExpr::Raw { .. });
            if raw {
                self.sql.push('(');
            }
            self.render_bool(expr);
            if raw {
                self.sql.push(')');
            }
        }
        self.sql.push(')');
    }
}
