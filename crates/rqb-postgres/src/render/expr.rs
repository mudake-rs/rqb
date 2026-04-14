use rqb_core::{
    FieldType, FunctionNameStyle, JsonAccessPath, LogicalOp, ValidatedExpr, ValidatedSqlExpr, Value,
};

use crate::Result;
use crate::helpers::write_quoted_ident;
use crate::type_sql::write_postgres_type_name;

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

    pub(super) fn render_sql_expr(&mut self, expr: &ValidatedSqlExpr) -> Result<()> {
        match expr {
            ValidatedSqlExpr::Field(field) => self.render_column_name(field),
            ValidatedSqlExpr::Excluded(field) => {
                self.sql.push_str("EXCLUDED.");
                write_quoted_ident(&mut self.sql, field.db_name.as_ref());
            }
            ValidatedSqlExpr::Value { value, ty } => self.push_typed_param(value, *ty),
            ValidatedSqlExpr::Raw { raw, .. } => self.render_raw(raw),
            ValidatedSqlExpr::Function {
                name,
                args,
                name_style,
                ..
            } => {
                match name_style {
                    FunctionNameStyle::Quoted => write_function_name(&mut self.sql, name),
                    FunctionNameStyle::Raw => self.sql.push_str(name),
                }
                self.sql.push('(');
                self.render_sql_expr_list(args)?;
                self.sql.push(')');
            }
            ValidatedSqlExpr::JsonAccess {
                expr, path, text, ..
            } => self.render_json_access(expr, path, *text)?,
            ValidatedSqlExpr::Window {
                function,
                args,
                spec,
                ..
            } => {
                self.sql.push_str(function.sql_name());
                self.sql.push('(');
                self.render_sql_expr_list(args)?;
                self.sql.push_str(") OVER (");
                if !spec.partition_by.is_empty() {
                    self.sql.push_str("PARTITION BY ");
                    for (idx, field) in spec.partition_by.iter().enumerate() {
                        if idx > 0 {
                            self.sql.push_str(", ");
                        }
                        self.render_column_name(field);
                    }
                }
                if !spec.order_by.is_empty() {
                    if !spec.partition_by.is_empty() {
                        self.sql.push(' ');
                    }
                    self.sql.push_str("ORDER BY ");
                    for (idx, sort) in spec.order_by.iter().enumerate() {
                        if idx > 0 {
                            self.sql.push_str(", ");
                        }
                        self.render_sort(sort);
                    }
                }
                self.sql.push(')');
            }
            ValidatedSqlExpr::Coalesce { args, .. } => {
                self.sql.push_str("COALESCE(");
                self.render_sql_expr_list(args)?;
                self.sql.push(')');
            }
            ValidatedSqlExpr::Case {
                branches,
                otherwise,
                ..
            } => {
                self.sql.push_str("CASE");
                for branch in branches {
                    self.sql.push_str(" WHEN ");
                    self.render_expr(&branch.condition)?;
                    self.sql.push_str(" THEN ");
                    self.render_sql_expr(&branch.value)?;
                }
                self.sql.push_str(" ELSE ");
                self.render_sql_expr(otherwise)?;
                self.sql.push_str(" END");
            }
            ValidatedSqlExpr::Cast { expr, ty } => {
                self.sql.push_str("CAST(");
                self.render_sql_expr(expr)?;
                self.sql.push_str(" AS ");
                write_postgres_type_name(&mut self.sql, *ty);
                self.sql.push(')');
            }
        }
        Ok(())
    }

    fn render_json_access(
        &mut self,
        expr: &ValidatedSqlExpr,
        path: &JsonAccessPath,
        text: bool,
    ) -> Result<()> {
        self.sql.push('(');
        self.render_sql_expr(expr)?;
        match path {
            JsonAccessPath::Key(key) => {
                self.sql.push_str(if text { " ->> " } else { " -> " });
                self.push_text_param(key);
            }
            JsonAccessPath::Index(index) => {
                self.sql.push_str(if text { " ->> " } else { " -> " });
                self.push_typed_param(&Value::I64(i64::from(*index)), FieldType::Integer);
            }
            JsonAccessPath::Path(path) => {
                self.sql.push_str(if text { " #>> " } else { " #> " });
                self.render_json_path(path);
            }
        }
        self.sql.push(')');
        Ok(())
    }

    fn render_sql_expr_list(&mut self, exprs: &[ValidatedSqlExpr]) -> Result<()> {
        for (idx, expr) in exprs.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_sql_expr(expr)?;
        }
        Ok(())
    }
}

fn write_function_name(output: &mut String, name: &str) {
    for (idx, part) in name.split('.').enumerate() {
        if idx > 0 {
            output.push('.');
        }
        write_quoted_ident(output, part);
    }
}
