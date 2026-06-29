use crate::Param;

use super::{
    BoolExpr, Field, FieldRef, ValueExpr, plainto_tsquery, to_tsvector, ts_match,
    websearch_to_tsquery,
};

macro_rules! field_text_delegates {
    ($($(#[$meta:meta])* $name:ident($arg:ident: $arg_ty:ty);)+) => {
        impl Field<String> {
            $(
                $(#[$meta])*
                pub fn $name(self, $arg: $arg_ty) -> BoolExpr {
                    self.reference().$name($arg)
                }
            )+
        }
    };
}

field_text_delegates! {
    /// Builds a `LIKE` predicate using PostgreSQL pattern syntax.
    like(pattern: impl AsRef<str>);
    /// Builds a `LIKE` predicate from a SQL pattern expression.
    like_expr(pattern: impl Into<ValueExpr>);
    /// Builds a negated `LIKE` predicate.
    not_like(pattern: impl AsRef<str>);
    /// Builds a negated `LIKE` predicate from a SQL pattern expression.
    not_like_expr(pattern: impl Into<ValueExpr>);
    /// Builds an `ILIKE` predicate using PostgreSQL pattern syntax.
    ilike(pattern: impl AsRef<str>);
    /// Builds an `ILIKE` predicate from a SQL pattern expression.
    ilike_expr(pattern: impl Into<ValueExpr>);
    /// Builds a negated `ILIKE` predicate.
    not_ilike(pattern: impl AsRef<str>);
    /// Builds a negated `ILIKE` predicate from a SQL pattern expression.
    not_ilike_expr(pattern: impl Into<ValueExpr>);
    /// Builds a `SIMILAR TO` predicate.
    similar_to(pattern: impl AsRef<str>);
    /// Builds a `SIMILAR TO` predicate from a SQL pattern expression.
    similar_to_expr(pattern: impl Into<ValueExpr>);
    /// Builds a negated `SIMILAR TO` predicate.
    not_similar_to(pattern: impl AsRef<str>);
    /// Builds a negated `SIMILAR TO` predicate from a SQL pattern expression.
    not_similar_to_expr(pattern: impl Into<ValueExpr>);
    /// Builds an escaped case-insensitive contains predicate for a literal value.
    contains(value: impl AsRef<str>);
    /// Builds an escaped case-insensitive contains predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    contains_expr(value: impl Into<ValueExpr>);
    /// Builds a negated escaped case-insensitive contains predicate.
    not_contains(value: impl AsRef<str>);
    /// Builds a negated escaped case-insensitive contains predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    not_contains_expr(value: impl Into<ValueExpr>);
    /// Builds an escaped case-insensitive prefix predicate for a literal value.
    starts_with(value: impl AsRef<str>);
    /// Builds an escaped case-insensitive prefix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    starts_with_expr(value: impl Into<ValueExpr>);
    /// Builds a negated escaped case-insensitive prefix predicate.
    not_starts_with(value: impl AsRef<str>);
    /// Builds a negated escaped case-insensitive prefix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    not_starts_with_expr(value: impl Into<ValueExpr>);
    /// Builds an escaped case-insensitive suffix predicate for a literal value.
    ends_with(value: impl AsRef<str>);
    /// Builds an escaped case-insensitive suffix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    ends_with_expr(value: impl Into<ValueExpr>);
    /// Builds a negated escaped case-insensitive suffix predicate.
    not_ends_with(value: impl AsRef<str>);
    /// Builds a negated escaped case-insensitive suffix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    not_ends_with_expr(value: impl Into<ValueExpr>);
    /// Builds a case-sensitive PostgreSQL regex predicate.
    regex(pattern: impl Into<String>);
    /// Builds a case-sensitive PostgreSQL regex predicate from a SQL pattern expression.
    regex_expr(pattern: impl Into<ValueExpr>);
    /// Builds a negated case-sensitive PostgreSQL regex predicate.
    not_regex(pattern: impl Into<String>);
    /// Builds a negated case-sensitive PostgreSQL regex predicate from a SQL pattern expression.
    not_regex_expr(pattern: impl Into<ValueExpr>);
    /// Builds a case-insensitive PostgreSQL regex predicate.
    iregex(pattern: impl Into<String>);
    /// Builds a case-insensitive PostgreSQL regex predicate from a SQL pattern expression.
    iregex_expr(pattern: impl Into<ValueExpr>);
    /// Builds a negated case-insensitive PostgreSQL regex predicate.
    not_iregex(pattern: impl Into<String>);
    /// Builds a negated case-insensitive PostgreSQL regex predicate from a SQL pattern expression.
    not_iregex_expr(pattern: impl Into<ValueExpr>);
    /// Builds a plain full-text search predicate.
    text_search(query: impl AsRef<str>);
    /// Builds a full-text search predicate from a tsquery expression.
    text_search_expr(query: impl Into<ValueExpr>);
    /// Builds a web-style full-text search predicate.
    websearch(query: impl AsRef<str>);
    /// Builds a web-style full-text search predicate from a text expression.
    websearch_expr(query: impl Into<ValueExpr>);
}

impl FieldRef<String> {
    /// Builds a `LIKE` predicate using PostgreSQL pattern syntax.
    pub fn like(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, false, false)
    }

    /// Builds a `LIKE` predicate from a SQL pattern expression.
    pub fn like_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.like_expr_predicate(pattern, false, false)
    }

    /// Builds a negated `LIKE` predicate.
    pub fn not_like(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, false, true)
    }

    /// Builds a negated `LIKE` predicate from a SQL pattern expression.
    pub fn not_like_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.like_expr_predicate(pattern, false, true)
    }

    /// Builds an `ILIKE` predicate using PostgreSQL pattern syntax.
    pub fn ilike(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, true, false)
    }

    /// Builds an `ILIKE` predicate from a SQL pattern expression.
    pub fn ilike_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.like_expr_predicate(pattern, true, false)
    }

    /// Builds a negated `ILIKE` predicate.
    pub fn not_ilike(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, true, true)
    }

    /// Builds a negated `ILIKE` predicate from a SQL pattern expression.
    pub fn not_ilike_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.like_expr_predicate(pattern, true, true)
    }

    /// Builds a `SIMILAR TO` predicate.
    pub fn similar_to(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.similar_predicate(pattern, false)
    }

    /// Builds a `SIMILAR TO` predicate from a SQL pattern expression.
    pub fn similar_to_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.similar_expr_predicate(pattern, false)
    }

    /// Builds a negated `SIMILAR TO` predicate.
    pub fn not_similar_to(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.similar_predicate(pattern, true)
    }

    /// Builds a negated `SIMILAR TO` predicate from a SQL pattern expression.
    pub fn not_similar_to_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.similar_expr_predicate(pattern, true)
    }

    /// Builds an escaped case-insensitive contains predicate for a literal value.
    pub fn contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", false)
    }

    /// Builds an escaped case-insensitive contains predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    pub fn contains_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.affix_expr_predicate(value, "%", "%", false)
    }

    /// Builds a negated escaped case-insensitive contains predicate.
    pub fn not_contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", true)
    }

    /// Builds a negated escaped case-insensitive contains predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    pub fn not_contains_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.affix_expr_predicate(value, "%", "%", true)
    }

    /// Builds an escaped case-insensitive prefix predicate for a literal value.
    pub fn starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "", "%", false)
    }

    /// Builds an escaped case-insensitive prefix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    pub fn starts_with_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.affix_expr_predicate(value, "", "%", false)
    }

    /// Builds a negated escaped case-insensitive prefix predicate.
    pub fn not_starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "", "%", true)
    }

    /// Builds a negated escaped case-insensitive prefix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    pub fn not_starts_with_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.affix_expr_predicate(value, "", "%", true)
    }

    /// Builds an escaped case-insensitive suffix predicate for a literal value.
    pub fn ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "", false)
    }

    /// Builds an escaped case-insensitive suffix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    pub fn ends_with_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.affix_expr_predicate(value, "%", "", false)
    }

    /// Builds a negated escaped case-insensitive suffix predicate.
    pub fn not_ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "", true)
    }

    /// Builds a negated escaped case-insensitive suffix predicate from a SQL expression.
    ///
    /// LIKE metacharacters (`%`, `_`, `\`) are escaped server-side with
    /// `replace(...)`, so the expression value is matched literally.
    pub fn not_ends_with_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.affix_expr_predicate(value, "%", "", true)
    }

    /// Builds a case-sensitive PostgreSQL regex predicate.
    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, false)
    }

    /// Builds a case-sensitive PostgreSQL regex predicate from a SQL pattern expression.
    pub fn regex_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.regex_expr_predicate(pattern, false, false)
    }

    /// Builds a negated case-sensitive PostgreSQL regex predicate.
    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, true)
    }

    /// Builds a negated case-sensitive PostgreSQL regex predicate from a SQL pattern expression.
    pub fn not_regex_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.regex_expr_predicate(pattern, false, true)
    }

    /// Builds a case-insensitive PostgreSQL regex predicate.
    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, false)
    }

    /// Builds a case-insensitive PostgreSQL regex predicate from a SQL pattern expression.
    pub fn iregex_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.regex_expr_predicate(pattern, true, false)
    }

    /// Builds a negated case-insensitive PostgreSQL regex predicate.
    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, true)
    }

    /// Builds a negated case-insensitive PostgreSQL regex predicate from a SQL pattern expression.
    pub fn not_iregex_expr(self, pattern: impl Into<ValueExpr>) -> BoolExpr {
        self.regex_expr_predicate(pattern, true, true)
    }

    /// Builds a plain full-text search predicate.
    pub fn text_search(self, query: impl AsRef<str>) -> BoolExpr {
        ts_match(
            to_tsvector(self.expr()),
            plainto_tsquery(ValueExpr::Param(Param::typed(query.as_ref().to_owned()))),
        )
    }

    /// Builds a full-text search predicate from a tsquery expression.
    pub fn text_search_expr(self, query: impl Into<ValueExpr>) -> BoolExpr {
        ts_match(to_tsvector(self.expr()), query)
    }

    /// Builds a web-style full-text search predicate.
    pub fn websearch(self, query: impl AsRef<str>) -> BoolExpr {
        ts_match(
            to_tsvector(self.expr()),
            websearch_to_tsquery(ValueExpr::Param(Param::typed(query.as_ref().to_owned()))),
        )
    }

    /// Builds a web-style full-text search predicate from a text expression.
    pub fn websearch_expr(self, query: impl Into<ValueExpr>) -> BoolExpr {
        ts_match(to_tsvector(self.expr()), websearch_to_tsquery(query))
    }

    fn like_predicate(
        self,
        pattern: impl AsRef<str>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::like(
            self.expr(),
            ValueExpr::Param(Param::typed(pattern.as_ref().to_owned())),
            case_insensitive,
            negated,
            false,
        )
    }

    fn like_expr_predicate(
        self,
        pattern: impl Into<ValueExpr>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::like(
            self.expr(),
            pattern.into(),
            case_insensitive,
            negated,
            false,
        )
    }

    fn similar_predicate(self, pattern: impl AsRef<str>, negated: bool) -> BoolExpr {
        BoolExpr::similar_to(
            self.expr(),
            ValueExpr::Param(Param::typed(pattern.as_ref().to_owned())),
            negated,
        )
    }

    fn similar_expr_predicate(self, pattern: impl Into<ValueExpr>, negated: bool) -> BoolExpr {
        BoolExpr::similar_to(self.expr(), pattern.into(), negated)
    }

    fn affix_predicate(
        self,
        value: impl AsRef<str>,
        prefix: &'static str,
        suffix: &'static str,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::like(
            self.expr(),
            ValueExpr::Param(Param::typed(escaped_like_pattern(
                value.as_ref(),
                prefix,
                suffix,
            ))),
            true,
            negated,
            true,
        )
    }

    fn affix_expr_predicate(
        self,
        value: impl Into<ValueExpr>,
        prefix: &'static str,
        suffix: &'static str,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::like(
            self.expr(),
            affix_like_pattern_expr(value, prefix, suffix),
            true,
            negated,
            true,
        )
    }

    fn regex_predicate(
        self,
        pattern: impl Into<String>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::regex(
            self.expr(),
            ValueExpr::Param(Param::typed(pattern.into())),
            case_insensitive,
            negated,
        )
    }

    fn regex_expr_predicate(
        self,
        pattern: impl Into<ValueExpr>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::regex(self.expr(), pattern.into(), case_insensitive, negated)
    }
}

pub(crate) fn escaped_like_pattern(value: &str, prefix: &str, suffix: &str) -> String {
    let mut escaped = String::with_capacity(prefix.len() + value.len() + suffix.len());
    escaped.push_str(prefix);
    if !value
        .bytes()
        .any(|byte| matches!(byte, b'\\' | b'%' | b'_'))
    {
        escaped.push_str(value);
        escaped.push_str(suffix);
        return escaped;
    }
    for ch in value.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped.push_str(suffix);
    escaped
}

fn affix_like_pattern_expr(
    value: impl Into<ValueExpr>,
    prefix: &'static str,
    suffix: &'static str,
) -> ValueExpr {
    let escaped = escape_like_expr(value);
    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => escaped,
        (false, true) => concat_text_param(prefix, escaped),
        (true, false) => concat_text_param(escaped, suffix),
        (false, false) => concat_text_param(concat_text_param(prefix, escaped), suffix),
    }
}

fn escape_like_expr(value: impl Into<ValueExpr>) -> ValueExpr {
    let escaped_backslash = replace_text(value, "\\", "\\\\");
    let escaped_percent = replace_text(escaped_backslash, "%", "\\%");
    replace_text(escaped_percent, "_", "\\_")
}

fn replace_text(
    value: impl Into<ValueExpr>,
    from: impl Into<ValueExpr>,
    to: impl Into<ValueExpr>,
) -> ValueExpr {
    super::function("replace", [value.into(), from.into(), to.into()])
}

fn concat_text_param(left: impl Into<ValueExpr>, right: impl Into<ValueExpr>) -> ValueExpr {
    super::concat_op(left, right)
}
