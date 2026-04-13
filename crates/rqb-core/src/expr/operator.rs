use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! impl_as_str {
    ($ty:ident { $($variant:ident => $value:expr),* $(,)? }) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)*
                }
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str((*self).as_str())
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operator {
    StartsWith,
    Contains,
    NotContains,
    EndsWith,
    Equals,
    NotEquals,
    In,
    NotIn,
    Lt,
    Lte,
    Gt,
    Gte,
    Between,
    NotBetween,
    IsNull,
    IsNotNull,
    NotStartsWith,
    NotEndsWith,
    IsDistinctFrom,
    IsNotDistinctFrom,
    ArrayContainsAny,
    ArrayContainsAll,
    ArrayElemMatch,
    ArrayContains,
    ArrayNotContains,
    ArrayIsEmpty,
    ArrayIsNotEmpty,
    JsonKeyExists,
    JsonKeysExistAny,
    JsonKeysExistAll,
    ContainedBy,
    Overlaps,
    Regex,
    NotRegex,
    TextSearch,
}

impl_as_str!(Operator {
    StartsWith => "startsWith",
    Contains => "contains",
    NotContains => "notContains",
    EndsWith => "endsWith",
    Equals => "equals",
    NotEquals => "notEquals",
    In => "in",
    NotIn => "notIn",
    Lt => "lt",
    Lte => "lte",
    Gt => "gt",
    Gte => "gte",
    Between => "between",
    NotBetween => "notBetween",
    IsNull => "isNull",
    IsNotNull => "isNotNull",
    NotStartsWith => "notStartsWith",
    NotEndsWith => "notEndsWith",
    IsDistinctFrom => "isDistinctFrom",
    IsNotDistinctFrom => "isNotDistinctFrom",
    ArrayContainsAny => "arrayContainsAny",
    ArrayContainsAll => "arrayContainsAll",
    ArrayElemMatch => "arrayElemMatch",
    ArrayContains => "arrayContains",
    ArrayNotContains => "arrayNotContains",
    ArrayIsEmpty => "arrayIsEmpty",
    ArrayIsNotEmpty => "arrayIsNotEmpty",
    JsonKeyExists => "jsonKeyExists",
    JsonKeysExistAny => "jsonKeysExistAny",
    JsonKeysExistAll => "jsonKeysExistAll",
    ContainedBy => "containedBy",
    Overlaps => "overlaps",
    Regex => "regex",
    NotRegex => "notRegex",
    TextSearch => "textSearch",
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorCategory {
    NullCheck,
    Equality,
    NullSafeEquality,
    Ordering,
    Inclusion,
    Between,
    Contains,
    TextAffix,
    Regex,
    ArraySet,
    ArrayMembership,
    ArrayState,
    ArrayElementMatch,
    JsonKey,
    JsonKeySet,
    Containment,
    TextSearch,
}

impl Operator {
    pub fn category(self) -> OperatorCategory {
        match self {
            Self::IsNull | Self::IsNotNull => OperatorCategory::NullCheck,
            Self::Equals | Self::NotEquals => OperatorCategory::Equality,
            Self::IsDistinctFrom | Self::IsNotDistinctFrom => OperatorCategory::NullSafeEquality,
            Self::Gt | Self::Gte | Self::Lt | Self::Lte => OperatorCategory::Ordering,
            Self::In | Self::NotIn => OperatorCategory::Inclusion,
            Self::Between | Self::NotBetween => OperatorCategory::Between,
            Self::Contains | Self::NotContains => OperatorCategory::Contains,
            Self::StartsWith | Self::EndsWith | Self::NotStartsWith | Self::NotEndsWith => {
                OperatorCategory::TextAffix
            }
            Self::Regex | Self::NotRegex => OperatorCategory::Regex,
            Self::ArrayContainsAny | Self::ArrayContainsAll => OperatorCategory::ArraySet,
            Self::ArrayContains | Self::ArrayNotContains => OperatorCategory::ArrayMembership,
            Self::ArrayIsEmpty | Self::ArrayIsNotEmpty => OperatorCategory::ArrayState,
            Self::ArrayElemMatch => OperatorCategory::ArrayElementMatch,
            Self::JsonKeyExists => OperatorCategory::JsonKey,
            Self::JsonKeysExistAny | Self::JsonKeysExistAll => OperatorCategory::JsonKeySet,
            Self::ContainedBy | Self::Overlaps => OperatorCategory::Containment,
            Self::TextSearch => OperatorCategory::TextSearch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColumnOperator {
    Equals,
    NotEquals,
    Lt,
    Lte,
    Gt,
    Gte,
}

impl_as_str!(ColumnOperator {
    Equals => "equals",
    NotEquals => "notEquals",
    Lt => "lt",
    Lte => "lte",
    Gt => "gt",
    Gte => "gte",
});

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{Operator, OperatorCategory};

    #[test]
    fn operators_report_stable_categories() {
        let cases = [
            (Operator::IsNull, OperatorCategory::NullCheck),
            (Operator::IsNotNull, OperatorCategory::NullCheck),
            (Operator::Equals, OperatorCategory::Equality),
            (Operator::NotEquals, OperatorCategory::Equality),
            (Operator::IsDistinctFrom, OperatorCategory::NullSafeEquality),
            (
                Operator::IsNotDistinctFrom,
                OperatorCategory::NullSafeEquality,
            ),
            (Operator::Gt, OperatorCategory::Ordering),
            (Operator::Gte, OperatorCategory::Ordering),
            (Operator::Lt, OperatorCategory::Ordering),
            (Operator::Lte, OperatorCategory::Ordering),
            (Operator::In, OperatorCategory::Inclusion),
            (Operator::NotIn, OperatorCategory::Inclusion),
            (Operator::Between, OperatorCategory::Between),
            (Operator::NotBetween, OperatorCategory::Between),
            (Operator::Contains, OperatorCategory::Contains),
            (Operator::NotContains, OperatorCategory::Contains),
            (Operator::StartsWith, OperatorCategory::TextAffix),
            (Operator::EndsWith, OperatorCategory::TextAffix),
            (Operator::NotStartsWith, OperatorCategory::TextAffix),
            (Operator::NotEndsWith, OperatorCategory::TextAffix),
            (Operator::Regex, OperatorCategory::Regex),
            (Operator::NotRegex, OperatorCategory::Regex),
            (Operator::ArrayContainsAny, OperatorCategory::ArraySet),
            (Operator::ArrayContainsAll, OperatorCategory::ArraySet),
            (Operator::ArrayContains, OperatorCategory::ArrayMembership),
            (
                Operator::ArrayNotContains,
                OperatorCategory::ArrayMembership,
            ),
            (Operator::ArrayIsEmpty, OperatorCategory::ArrayState),
            (Operator::ArrayIsNotEmpty, OperatorCategory::ArrayState),
            (
                Operator::ArrayElemMatch,
                OperatorCategory::ArrayElementMatch,
            ),
            (Operator::JsonKeyExists, OperatorCategory::JsonKey),
            (Operator::JsonKeysExistAny, OperatorCategory::JsonKeySet),
            (Operator::JsonKeysExistAll, OperatorCategory::JsonKeySet),
            (Operator::ContainedBy, OperatorCategory::Containment),
            (Operator::Overlaps, OperatorCategory::Containment),
            (Operator::TextSearch, OperatorCategory::TextSearch),
        ];

        for (operator, category) in cases {
            assert_eq!(operator.category(), category, "{operator}");
        }
    }
}
