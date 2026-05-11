use super::*;

impl Renderer {
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
            ValueExpr::Keyword(keyword) => {
                self.sql.push_str(keyword);
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
            ValueExpr::OrderedSetAggregate {
                name,
                args,
                within_group,
                filter,
            } => {
                self.render_ordered_set_aggregate(name, args, within_group)?;
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
                self.sql.push_str(op.as_sql());
                self.sql.push(' ');
                self.render_value(right)?;
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Subscript { expr, index } => {
                self.render_value(expr)?;
                self.sql.push('[');
                self.render_value(index)?;
                self.sql.push(']');
                Ok(())
            }
            ValueExpr::Slice { expr, start, end } => {
                self.render_value(expr)?;
                self.sql.push('[');
                if let Some(start) = start {
                    self.render_value(start)?;
                }
                self.sql.push(':');
                if let Some(end) = end {
                    self.render_value(end)?;
                }
                self.sql.push(']');
                Ok(())
            }
            ValueExpr::Array(values) => {
                self.sql.push_str("ARRAY[");
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_value(value)?;
                }
                self.sql.push(']');
                Ok(())
            }
            ValueExpr::Row(values) => {
                self.sql.push_str("ROW(");
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_value(value)?;
                }
                self.sql.push(')');
                Ok(())
            }
            ValueExpr::Extract { field, expr } => {
                self.sql.push_str("extract(");
                self.sql.push_str(field);
                self.sql.push_str(" FROM ");
                self.render_value(expr)?;
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
                        if let Some(nulls) = item.nulls {
                            self.sql.push(' ');
                            self.sql.push_str(nulls.as_sql());
                        }
                    }
                    needs_space = true;
                }
                if let Some(frame) = &spec.frame {
                    if needs_space {
                        self.sql.push(' ');
                    }
                    self.render_window_frame(frame)?;
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
        order_by: &[crate::OrderItem],
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
                if let Some(nulls) = item.nulls {
                    self.sql.push(' ');
                    self.sql.push_str(nulls.as_sql());
                }
            }
        }
        self.sql.push(')');
        Ok(())
    }

    pub(super) fn render_ordered_set_aggregate(
        &mut self,
        name: &str,
        args: &[ValueExpr],
        within_group: &[crate::OrderItem],
    ) -> Result<()> {
        self.sql.push_str(name);
        self.sql.push('(');
        for (idx, arg) in args.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(arg)?;
        }
        self.sql.push_str(") WITHIN GROUP (ORDER BY ");
        for (idx, item) in within_group.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_value(&item.expr)?;
            self.sql.push(' ');
            self.sql.push_str(item.direction.as_sql());
            if let Some(nulls) = item.nulls {
                self.sql.push(' ');
                self.sql.push_str(nulls.as_sql());
            }
        }
        self.sql.push(')');
        Ok(())
    }

    pub(super) fn render_window_frame(&mut self, frame: &WindowFrame) -> Result<()> {
        self.sql.push_str(frame.kind.as_sql());
        if let Some(end) = &frame.end {
            self.sql.push_str(" BETWEEN ");
            self.render_frame_bound(&frame.start)?;
            self.sql.push_str(" AND ");
            self.render_frame_bound(end)?;
        } else {
            self.sql.push(' ');
            self.render_frame_bound(&frame.start)?;
        }
        if let Some(exclude) = frame.exclude {
            self.sql.push(' ');
            self.sql.push_str(exclude.as_sql());
        }
        Ok(())
    }

    pub(super) fn render_frame_bound(&mut self, bound: &FrameBound) -> Result<()> {
        match bound {
            FrameBound::UnboundedPreceding => self.sql.push_str("UNBOUNDED PRECEDING"),
            FrameBound::Preceding(expr) => {
                self.render_value(expr)?;
                self.sql.push_str(" PRECEDING");
            }
            FrameBound::CurrentRow => self.sql.push_str("CURRENT ROW"),
            FrameBound::Following(expr) => {
                self.render_value(expr)?;
                self.sql.push_str(" FOLLOWING");
            }
            FrameBound::UnboundedFollowing => self.sql.push_str("UNBOUNDED FOLLOWING"),
        }
        Ok(())
    }
}
