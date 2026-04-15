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

    pub(super) fn render_value(&mut self, expr: &ValueExpr) -> Result<()> {
        match expr {
            ValueExpr::Field { meta, qualifier } => {
                self.render_field(meta, qualifier.as_deref());
                Ok(())
            }
            ValueExpr::Excluded(field) => {
                self.sql.push_str("EXCLUDED.");
                write_quoted_ident(&mut self.sql, field.db);
                Ok(())
            }
            ValueExpr::Param(param) => {
                self.push_param(param.clone());
                Ok(())
            }
            ValueExpr::Function { name, args } => self.render_call(name, args),
            ValueExpr::Aggregate {
                name,
                args,
                distinct,
                order_by,
                filter,
            } => {
                self.render_aggregate(name, args, *distinct, order_by)?;
                if let Some(filter) = filter {
                    self.sql.push_str(" FILTER (WHERE ");
                    self.render_bool(filter)?;
                    self.sql.push(')');
                }
                Ok(())
            }
            ValueExpr::Case { branches, else_ } => {
                self.sql.push_str("CASE");
                for (when, then) in branches {
                    self.sql.push_str(" WHEN ");
                    self.render_bool(when)?;
                    self.sql.push_str(" THEN ");
                    self.render_value(then)?;
                }
                if let Some(else_) = else_ {
                    self.sql.push_str(" ELSE ");
                    self.render_value(else_)?;
                }
                self.sql.push_str(" END");
                Ok(())
            }
            ValueExpr::Cast { expr, pg } => {
                self.sql.push_str("CAST(");
                self.render_value(expr)?;
                self.sql.push_str(" AS ");
                self.sql.push_str(pg);
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Binary { left, op, right } => {
                self.sql.push('(');
                self.render_value(left)?;
                self.sql.push(' ');
                self.sql.push_str(value_op_sql(*op));
                self.sql.push(' ');
                self.render_value(right)?;
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Window {
                function,
                args,
                spec,
            } => {
                self.sql.push_str(function.as_sql());
                self.sql.push('(');
                for (idx, arg) in args.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_value(arg)?;
                }
                self.sql.push_str(") OVER (");
                let mut needs_space = false;
                if !spec.partition_by.is_empty() {
                    self.sql.push_str("PARTITION BY ");
                    for (idx, expr) in spec.partition_by.iter().enumerate() {
                        if idx > 0 {
                            self.sql.push_str(", ");
                        }
                        self.render_value(expr)?;
                    }
                    needs_space = true;
                }
                if !spec.order_by.is_empty() {
                    if needs_space {
                        self.sql.push(' ');
                    }
                    self.sql.push_str("ORDER BY ");
                    for (idx, item) in spec.order_by.iter().enumerate() {
                        if idx > 0 {
                            self.sql.push_str(", ");
                        }
                        self.render_value(&item.expr)?;
                        self.sql.push(' ');
                        self.sql.push_str(item.direction.as_sql());
                    }
                }
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Raw { sql, params } => self.render_raw(sql, params),
            ValueExpr::Subquery(stmt) => {
                self.sql.push('(');
                self.render_stmt(stmt)?;
                self.sql.push(')');
                Ok(())
            }
        }
    }

    pub(super) fn render_call(&mut self, name: &str, args: &[ValueExpr]) -> Result<()> {
        self.sql.push_str(name);
        self.sql.push('(');
        for (idx, arg) in args.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(arg)?;
        }
        self.sql.push(')');
        Ok(())
    }

    pub(super) fn render_aggregate(
        &mut self,
        name: &str,
        args: &[ValueExpr],
        distinct: bool,
        order_by: &[crate::typed::OrderItem],
    ) -> Result<()> {
        self.sql.push_str(name);
        self.sql.push('(');
        if distinct {
            self.sql.push_str("DISTINCT ");
        }
        if args.is_empty() {
            self.sql.push('*');
        } else {
            for (idx, arg) in args.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.render_value(arg)?;
            }
        }
        if !order_by.is_empty() {
            self.sql.push_str(" ORDER BY ");
            for (idx, item) in order_by.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.render_value(&item.expr)?;
                self.sql.push(' ');
                self.sql.push_str(item.direction.as_sql());
            }
        }
        self.sql.push(')');
        Ok(())
    }
}

pub(super) fn value_op_sql(op: ValueOp) -> &'static str {
    match op {
        ValueOp::Add => "+",
        ValueOp::Sub => "-",
        ValueOp::Mul => "*",
        ValueOp::Div => "/",
        ValueOp::Custom(op) => op,
    }
}
