use crate::typed::Param;

use super::{BoolExpr, Field, FieldRef, ValueExpr};

impl Field<String> {
    pub fn like(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().like(pattern)
    }

    pub fn not_like(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_like(pattern)
    }

    pub fn ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().ilike(pattern)
    }

    pub fn not_ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_ilike(pattern)
    }

    pub fn contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().contains(value)
    }

    pub fn not_contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().not_contains(value)
    }

    pub fn starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().starts_with(value)
    }

    pub fn not_starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().not_starts_with(value)
    }

    pub fn ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().ends_with(value)
    }

    pub fn not_ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.reference().not_ends_with(value)
    }

    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().regex(pattern)
    }

    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_regex(pattern)
    }

    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().iregex(pattern)
    }

    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_iregex(pattern)
    }
}

impl FieldRef<String> {
    pub fn like(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, false, false)
    }

    pub fn not_like(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, false, true)
    }

    pub fn ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, true, false)
    }

    pub fn not_ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, true, true)
    }

    pub fn contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", false)
    }

    pub fn not_contains(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", true)
    }

    pub fn starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "", "%", false)
    }

    pub fn not_starts_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "", "%", true)
    }

    pub fn ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "", false)
    }

    pub fn not_ends_with(self, value: impl AsRef<str>) -> BoolExpr {
        self.affix_predicate(value, "%", "", true)
    }

    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, false)
    }

    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, true)
    }

    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, false)
    }

    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, true)
    }

    fn like_predicate(
        self,
        pattern: impl Into<String>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Like {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(pattern.into())),
            case_insensitive,
            negated,
            escape: false,
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
