use crate::typed::Param;

use super::{
    BoolExpr, Field, FieldRef, ValueExpr, plainto_tsquery, to_tsvector, ts_match,
    websearch_to_tsquery,
};

impl Field<String> {
    /// Builds a `LIKE` predicate.
    pub fn like(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.reference().like(pattern)
    }

    /// Builds a negated `LIKE` predicate.
    pub fn not_like(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.reference().not_like(pattern)
    }

    /// Builds an `ILIKE` predicate.
    pub fn ilike(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.reference().ilike(pattern)
    }

    /// Builds a negated `ILIKE` predicate.
    pub fn not_ilike(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.reference().not_ilike(pattern)
    }

    /// Builds a `SIMILAR TO` predicate.
    pub fn similar_to(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.reference().similar_to(pattern)
    }

    /// Builds a negated `SIMILAR TO` predicate.
    pub fn not_similar_to(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.reference().not_similar_to(pattern)
    }

    /// Builds an escaped case-insensitive contains predicate.
    pub fn contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().contains(value)
    }

    /// Builds a negated escaped case-insensitive contains predicate.
    pub fn not_contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().not_contains(value)
    }

    /// Builds an escaped case-insensitive prefix predicate.
    pub fn starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().starts_with(value)
    }

    /// Builds a negated escaped case-insensitive prefix predicate.
    pub fn not_starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().not_starts_with(value)
    }

    /// Builds an escaped case-insensitive suffix predicate.
    pub fn ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().ends_with(value)
    }

    /// Builds a negated escaped case-insensitive suffix predicate.
    pub fn not_ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().not_ends_with(value)
    }

    /// Builds a case-sensitive PostgreSQL regex predicate.
    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().regex(pattern)
    }

    /// Builds a negated case-sensitive PostgreSQL regex predicate.
    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_regex(pattern)
    }

    /// Builds a case-insensitive PostgreSQL regex predicate.
    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().iregex(pattern)
    }

    /// Builds a negated case-insensitive PostgreSQL regex predicate.
    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_iregex(pattern)
    }

    /// Builds a plain full-text search predicate.
    pub fn text_search(self, query: impl AsRef<str>) -> BoolExpr {
        self.reference().text_search(query)
    }

    /// Builds a web-style full-text search predicate.
    pub fn websearch(self, query: impl AsRef<str>) -> BoolExpr {
        self.reference().websearch(query)
    }
}

impl FieldRef<String> {
    /// Builds a `LIKE` predicate.
    pub fn like(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, false, false)
    }

    /// Builds a negated `LIKE` predicate.
    pub fn not_like(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, false, true)
    }

    /// Builds an `ILIKE` predicate.
    pub fn ilike(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, true, false)
    }

    /// Builds a negated `ILIKE` predicate.
    pub fn not_ilike(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.like_predicate(pattern, true, true)
    }

    /// Builds a `SIMILAR TO` predicate.
    pub fn similar_to(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.similar_predicate(pattern, false)
    }

    /// Builds a negated `SIMILAR TO` predicate.
    pub fn not_similar_to(self, pattern: impl AsRef<str>) -> BoolExpr {
        self.similar_predicate(pattern, true)
    }

    /// Builds an escaped case-insensitive contains predicate.
    pub fn contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", false)
    }

    /// Builds a negated escaped case-insensitive contains predicate.
    pub fn not_contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", true)
    }

    /// Builds an escaped case-insensitive prefix predicate.
    pub fn starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "", "%", false)
    }

    /// Builds a negated escaped case-insensitive prefix predicate.
    pub fn not_starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "", "%", true)
    }

    /// Builds an escaped case-insensitive suffix predicate.
    pub fn ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "", false)
    }

    /// Builds a negated escaped case-insensitive suffix predicate.
    pub fn not_ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "", true)
    }

    /// Builds a case-sensitive PostgreSQL regex predicate.
    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, false)
    }

    /// Builds a negated case-sensitive PostgreSQL regex predicate.
    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, true)
    }

    /// Builds a case-insensitive PostgreSQL regex predicate.
    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, false)
    }

    /// Builds a negated case-insensitive PostgreSQL regex predicate.
    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, true)
    }

    /// Builds a plain full-text search predicate.
    pub fn text_search(self, query: impl AsRef<str>) -> BoolExpr {
        ts_match(
            to_tsvector(self.expr()),
            plainto_tsquery(ValueExpr::Param(Param::typed(query.as_ref().to_owned()))),
        )
    }

    /// Builds a web-style full-text search predicate.
    pub fn websearch(self, query: impl AsRef<str>) -> BoolExpr {
        ts_match(
            to_tsvector(self.expr()),
            websearch_to_tsquery(ValueExpr::Param(Param::typed(query.as_ref().to_owned()))),
        )
    }

    fn like_predicate(
        self,
        pattern: impl AsRef<str>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Like {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(pattern.as_ref().to_owned())),
            case_insensitive,
            negated,
            escape: false,
        }
    }

    fn similar_predicate(self, pattern: impl AsRef<str>, negated: bool) -> BoolExpr {
        BoolExpr::SimilarTo {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(pattern.as_ref().to_owned())),
            negated,
        }
    }

    fn affix_predicate(
        self,
        value: impl AsRef<str>,
        prefix: &'static str,
        suffix: &'static str,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Like {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(escaped_like_pattern(
                value.as_ref(),
                prefix,
                suffix,
            ))),
            case_insensitive: true,
            negated,
            escape: true,
        }
    }

    fn regex_predicate(
        self,
        pattern: impl Into<String>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Regex {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(pattern.into())),
            case_insensitive,
            negated,
        }
    }
}

pub(crate) fn escaped_like_pattern(value: &str, prefix: &str, suffix: &str) -> String {
    let mut escaped = String::with_capacity(prefix.len() + value.len() + suffix.len());
    escaped.push_str(prefix);
    for ch in value.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped.push_str(suffix);
    escaped
}
