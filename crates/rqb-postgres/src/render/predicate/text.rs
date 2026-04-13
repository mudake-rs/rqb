use rqb_core::{ResolvedField, TextSearchConfig, ValidatedLikePattern, Value};

use crate::helpers::{escape_like, quote_literal};

use super::super::Renderer;

impl Renderer {
    pub(super) fn render_like_predicate(
        &mut self,
        field: &ResolvedField,
        pattern: ValidatedLikePattern,
        value: &str,
        negated: bool,
    ) {
        let (prefix, suffix) = like_bounds(pattern);
        self.render_like(field, value, prefix, suffix, negated);
    }

    pub(super) fn render_regex_predicate(
        &mut self,
        field: &ResolvedField,
        value: &str,
        negated: bool,
    ) {
        self.render_text_target(field);
        if negated {
            self.sql.push_str(" !~* ");
        } else {
            self.sql.push_str(" ~* ");
        }
        self.push_owned_param(Value::String(value.to_owned()));
    }

    pub(super) fn render_text_search(&mut self, field: &ResolvedField, value: &str) {
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
        self.push_owned_param(Value::String(value.to_owned()));
        self.sql.push(')');
    }

    fn render_like(
        &mut self,
        field: &ResolvedField,
        value: &str,
        prefix: &str,
        suffix: &str,
        negate: bool,
    ) {
        self.render_text_target(field);
        self.sql.push(' ');
        if negate {
            self.sql.push_str("NOT ");
        }
        self.sql.push_str("ILIKE ");
        let pattern = format!("{prefix}{}{suffix}", escape_like(value));
        self.push_owned_param(Value::String(pattern));
        self.sql.push_str(" ESCAPE '\\'");
    }
}

fn like_bounds(pattern: ValidatedLikePattern) -> (&'static str, &'static str) {
    match pattern {
        ValidatedLikePattern::Contains => ("%", "%"),
        ValidatedLikePattern::StartsWith => ("", "%"),
        ValidatedLikePattern::EndsWith => ("%", ""),
    }
}
