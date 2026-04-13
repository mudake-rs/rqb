use rqb_core::{
    ColumnOperator, ElemType, FieldType, LogicalOp, Operator, ResolvedField, TextSearchConfig,
    ValidatedExpr, Value,
};

use crate::Result;
use crate::helpers::{
    column_operator_sql, escape_like, quote_literal, value_to_json, value_to_json_array,
};
use crate::type_sql::array_element_field_type;

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

    fn render_predicate(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) -> Result<()> {
        use Operator::*;

        match operator {
            IsNull => {
                if field.is_json_path() {
                    self.render_text_target(field);
                } else {
                    self.render_column_name(field);
                }
                self.sql.push_str(" IS NULL");
            }
            IsNotNull => {
                if field.is_json_path() {
                    self.render_text_target(field);
                } else {
                    self.render_column_name(field);
                }
                self.sql.push_str(" IS NOT NULL");
            }
            Contains if field.ty.is_range() || field.ty.is_network() => {
                self.render_contains(field, value, false)
            }
            NotContains if field.ty.is_range() || field.ty.is_network() => {
                self.render_contains(field, value, true)
            }
            Contains => self.render_like(field, value, "%", "%", false),
            NotContains => self.render_like(field, value, "%", "%", true),
            StartsWith => self.render_like(field, value, "", "%", false),
            EndsWith => self.render_like(field, value, "%", "", false),
            NotStartsWith => self.render_like(field, value, "", "%", true),
            NotEndsWith => self.render_like(field, value, "%", "", true),
            Equals => self.render_binary(field, "=", value),
            NotEquals => self.render_binary(field, "<>", value),
            IsDistinctFrom => self.render_null_safe_binary(field, "IS DISTINCT FROM", value),
            IsNotDistinctFrom => self.render_null_safe_binary(field, "IS NOT DISTINCT FROM", value),
            Gt => self.render_binary(field, ">", value),
            Gte => self.render_binary(field, ">=", value),
            Lt => self.render_binary(field, "<", value),
            Lte => self.render_binary(field, "<=", value),
            In => self.render_in(field, value),
            NotIn => self.render_not_in(field, value),
            Between => self.render_between(field, value),
            NotBetween => self.render_not_between(field, value),
            ArrayContainsAny => {
                self.render_column_name(field);
                self.sql.push_str(" && ");
                self.push_typed_param(value, field.ty);
            }
            ArrayContainsAll => {
                self.render_column_name(field);
                self.sql.push_str(" @> ");
                self.push_typed_param(value, field.ty);
            }
            ArrayElemMatch => {
                if field.ty.is_array() && !field.is_json_path() {
                    self.render_column_name(field);
                    self.sql.push_str(" @> ");
                    self.push_typed_param(&Value::Array(vec![value.clone()]), field.ty);
                } else {
                    if field.is_json_path() {
                        self.render_json_target(field);
                    } else {
                        self.render_column_name(field);
                    }
                    self.sql.push_str(" @> ");
                    self.push_typed_param(&value_to_json_array(value), FieldType::Jsonb);
                }
            }
            ArrayContains => self.render_array_contains(field, value, false),
            ArrayNotContains => self.render_array_contains(field, value, true),
            ArrayIsEmpty => {
                self.sql.push_str("cardinality(");
                self.render_column_name(field);
                self.sql.push_str(") = 0");
            }
            ArrayIsNotEmpty => {
                self.sql.push_str("cardinality(");
                self.render_column_name(field);
                self.sql.push_str(") > 0");
            }
            JsonKeyExists => {
                self.render_column_name(field);
                self.sql.push_str(" ? ");
                self.push_param(value);
            }
            JsonKeysExistAny => {
                self.render_column_name(field);
                self.sql.push_str(" ?| ");
                self.push_typed_param(value, FieldType::Array(ElemType::Text));
            }
            JsonKeysExistAll => {
                self.render_column_name(field);
                self.sql.push_str(" ?& ");
                self.push_typed_param(value, FieldType::Array(ElemType::Text));
            }
            ContainedBy => self.render_contained_by(field, value),
            Overlaps => self.render_overlaps(field, value),
            Regex => {
                self.render_text_target(field);
                self.sql.push_str(" ~* ");
                self.push_param(value);
            }
            NotRegex => {
                self.render_text_target(field);
                self.sql.push_str(" !~* ");
                self.push_param(value);
            }
            TextSearch => self.render_text_search(field, value),
        }
        Ok(())
    }

    fn render_column_predicate(
        &mut self,
        left: &ResolvedField,
        operator: ColumnOperator,
        right: &ResolvedField,
    ) -> Result<()> {
        self.render_column_compare_target(left, operator);
        self.sql.push(' ');
        self.sql.push_str(column_operator_sql(operator));
        self.sql.push(' ');
        self.render_column_compare_target(right, operator);
        Ok(())
    }

    fn render_like(
        &mut self,
        field: &ResolvedField,
        value: &Value,
        prefix: &str,
        suffix: &str,
        negate: bool,
    ) {
        let text = match value {
            Value::String(value) => value,
            _ => unreachable!("validated by rqb-core"),
        };
        self.render_text_target(field);
        self.sql.push(' ');
        if negate {
            self.sql.push_str("NOT ");
        }
        self.sql.push_str("ILIKE ");
        let pattern = format!("{prefix}{}{suffix}", escape_like(text));
        self.push_param(&Value::String(pattern));
        self.sql.push_str(" ESCAPE '\\'");
    }

    fn render_contains(&mut self, field: &ResolvedField, value: &Value, negate: bool) {
        if negate {
            self.sql.push_str("NOT (");
        }
        self.render_column_name(field);
        if field.ty.is_network() {
            self.sql.push_str(" >>= ");
        } else {
            self.sql.push_str(" @> ");
        }
        self.push_typed_param(value, field.ty);
        if negate {
            self.sql.push(')');
        }
    }

    fn render_contained_by(&mut self, field: &ResolvedField, value: &Value) {
        self.render_column_name(field);
        if field.ty.is_network() {
            self.sql.push_str(" <<= ");
        } else {
            self.sql.push_str(" <@ ");
        }
        self.push_typed_param(value, field.ty);
    }

    fn render_overlaps(&mut self, field: &ResolvedField, value: &Value) {
        self.render_column_name(field);
        self.sql.push_str(" && ");
        self.push_typed_param(value, field.ty);
    }

    fn render_binary(&mut self, field: &ResolvedField, op: &str, value: &Value) {
        if field.is_json_path() {
            match op {
                "=" | "<>" => {
                    self.render_json_target(field);
                    self.sql.push(' ');
                    self.sql.push_str(op);
                    self.sql.push(' ');
                    self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
                }
                _ => {
                    self.sql.push('(');
                    self.render_text_target(field);
                    self.sql.push_str(")::numeric ");
                    self.sql.push_str(op);
                    self.sql.push(' ');
                    self.push_typed_param(value, FieldType::Numeric);
                }
            }
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        if field.ty.is_jsonb() {
            self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
        } else {
            self.push_typed_param(value, field.ty);
        }
    }

    fn render_null_safe_binary(&mut self, field: &ResolvedField, op: &str, value: &Value) {
        if field.is_json_path() {
            if value.is_null() {
                self.render_text_target(field);
                self.sql.push(' ');
                self.sql.push_str(op);
                self.sql.push(' ');
                self.push_param(value);
                return;
            }

            self.render_json_target(field);
            self.sql.push(' ');
            self.sql.push_str(op);
            self.sql.push(' ');
            self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        if field.ty.is_jsonb() && !value.is_null() {
            self.push_typed_param(&value_to_json(value), FieldType::Jsonb);
        } else {
            self.push_typed_param(value, field.ty);
        }
    }

    fn render_in(&mut self, field: &ResolvedField, value: &Value) {
        let Value::Array(values) = value else {
            unreachable!("validated by rqb-core");
        };
        if values.is_empty() {
            self.sql.push_str("FALSE");
            return;
        }

        if field.is_json_path() {
            self.render_json_target(field);
            self.sql.push_str(" = ANY(");
            self.push_jsonb_array_param(value);
            self.sql.push(')');
            return;
        }

        self.render_column_name(field);
        self.sql.push_str(" = ANY(");
        if field.ty.is_jsonb() {
            self.push_jsonb_array_param(value);
        } else {
            self.push_scalar_array_param(value, field.ty);
        }
        self.sql.push(')');
    }

    fn render_not_in(&mut self, field: &ResolvedField, value: &Value) {
        let Value::Array(values) = value else {
            unreachable!("validated by rqb-core");
        };
        if values.is_empty() {
            self.sql.push_str("TRUE");
            return;
        }

        self.sql.push_str("NOT (");
        self.render_in(field, value);
        self.sql.push(')');
    }

    fn render_between(&mut self, field: &ResolvedField, value: &Value) {
        self.render_between_op(field, value, "BETWEEN")
    }

    fn render_not_between(&mut self, field: &ResolvedField, value: &Value) {
        self.render_between_op(field, value, "NOT BETWEEN")
    }

    fn render_between_op(&mut self, field: &ResolvedField, value: &Value, op: &str) {
        let Value::Array(values) = value else {
            unreachable!("validated by rqb-core");
        };
        if field.is_json_path() {
            self.sql.push('(');
            self.render_text_target(field);
            self.sql.push_str(")::numeric ");
            self.sql.push_str(op);
            self.sql.push(' ');
            self.push_typed_param(&values[0], FieldType::Numeric);
            self.sql.push_str(" AND ");
            self.push_typed_param(&values[1], FieldType::Numeric);
            return;
        }

        self.render_column_name(field);
        self.sql.push(' ');
        self.sql.push_str(op);
        self.sql.push(' ');
        self.push_typed_param(&values[0], field.ty);
        self.sql.push_str(" AND ");
        self.push_typed_param(&values[1], field.ty);
    }

    fn render_array_contains(&mut self, field: &ResolvedField, value: &Value, negate: bool) {
        if negate {
            self.sql.push_str("NOT (");
        }
        self.push_typed_param(value, array_element_field_type(field.ty));
        self.sql.push_str(" = ANY(");
        self.render_column_name(field);
        self.sql.push(')');
        if negate {
            self.sql.push(')');
        }
    }

    fn render_text_search(&mut self, field: &ResolvedField, value: &Value) {
        let TextSearchConfig::Config(config) = field.caps.text_search else {
            unreachable!("validated by rqb-core");
        };
        self.sql.push_str("to_tsvector(");
        self.sql.push_str(&quote_literal(config));
        self.sql.push_str(", ");
        self.render_text_target(field);
        self.sql.push_str(") @@ websearch_to_tsquery(");
        self.sql.push_str(&quote_literal(config));
        self.sql.push_str(", ");
        self.push_param(value);
        self.sql.push(')');
    }

    fn render_text_target(&mut self, field: &ResolvedField) {
        if field.is_json_path() {
            self.render_column_name(field);
            self.sql.push_str(" #>> ");
            self.render_json_path(&field.json_path);
        } else {
            self.render_column_name(field);
            if field.ty != FieldType::Text {
                self.sql.push_str("::text");
            }
        }
    }

    fn render_json_target(&mut self, field: &ResolvedField) {
        if field.is_json_path() {
            self.render_column_name(field);
            self.sql.push_str(" #> ");
            self.render_json_path(&field.json_path);
        } else {
            self.render_column_name(field);
        }
    }

    fn render_column_compare_target(&mut self, field: &ResolvedField, operator: ColumnOperator) {
        if field.is_json_path() {
            if matches!(operator, ColumnOperator::Equals | ColumnOperator::NotEquals) {
                self.render_json_target(field);
            } else {
                self.render_text_target(field);
            }
        } else {
            self.render_column_name(field);
        }
    }
}
