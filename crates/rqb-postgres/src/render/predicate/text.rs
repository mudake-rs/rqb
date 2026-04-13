use rqb_core::{Operator, ResolvedField, TextSearchConfig, Value};

use crate::helpers::{escape_like, quote_literal};

use super::super::Renderer;

impl Renderer {
    pub(super) fn render_contains_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        let negated = match operator {
            Operator::Contains => false,
            Operator::NotContains => true,
            _ => unreachable!("operator category validated by Operator::category"),
        };
        if field.ty.is_range() || field.ty.is_network() {
            self.render_contains(field, value, negated);
        } else {
            self.render_like(field, value, "%", "%", negated);
        }
    }

    pub(super) fn render_text_affix_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        match operator {
            Operator::StartsWith => self.render_like(field, value, "", "%", false),
            Operator::EndsWith => self.render_like(field, value, "%", "", false),
            Operator::NotStartsWith => self.render_like(field, value, "", "%", true),
            Operator::NotEndsWith => self.render_like(field, value, "%", "", true),
            _ => unreachable!("operator category validated by Operator::category"),
        }
    }

    pub(super) fn render_regex_operator(
        &mut self,
        field: &ResolvedField,
        operator: Operator,
        value: &Value,
    ) {
        self.render_text_target(field);
        match operator {
            Operator::Regex => self.sql.push_str(" ~* "),
            Operator::NotRegex => self.sql.push_str(" !~* "),
            _ => unreachable!("operator category validated by Operator::category"),
        }
        self.push_param(value);
    }

    pub(super) fn render_text_search(&mut self, field: &ResolvedField, value: &Value) {
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
}
