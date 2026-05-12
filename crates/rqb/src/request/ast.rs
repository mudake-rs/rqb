use serde::Deserialize;
use serde_json::Value as JsonValue;

/// Client-supplied filtering, sorting, and pagination for a server-owned query.
///
/// A request is compiled against the source metadata of a trusted [`Select`].
/// Fields must exist in that source and be exposed with [`Meta::json`]. Invalid
/// client input returns structured search errors such as
/// [`Error::InvalidSearchField`], [`Error::SearchFieldNotExposed`],
/// [`Error::InvalidSearchOperator`], [`Error::InvalidSearchValue`],
/// [`Error::InvalidSort`], and [`Error::EmptySearchLogical`].
///
/// [`Error::EmptySearchLogical`]: crate::Error::EmptySearchLogical
/// [`Error::InvalidSearchField`]: crate::Error::InvalidSearchField
/// [`Error::InvalidSearchOperator`]: crate::Error::InvalidSearchOperator
/// [`Error::InvalidSearchValue`]: crate::Error::InvalidSearchValue
/// [`Error::InvalidSort`]: crate::Error::InvalidSort
/// [`Error::SearchFieldNotExposed`]: crate::Error::SearchFieldNotExposed
/// [`Meta::json`]: crate::Meta::json
/// [`Select`]: crate::Select
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// Optional boolean filter tree.
    #[serde(default)]
    pub filter: Option<SearchFilter>,
    /// Sort keys applied in order.
    #[serde(default)]
    pub sort: Vec<SearchSort>,
    /// Maximum number of rows to return.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Number of rows to skip before returning results.
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Boolean filter tree accepted by [`SearchRequest`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SearchFilter {
    /// Conjunction of nested filters.
    And(Vec<SearchFilter>),
    /// Disjunction of nested filters.
    Or(Vec<SearchFilter>),
    /// Negation of a nested filter.
    Not(Box<SearchFilter>),
    /// A field/operator/value predicate.
    Predicate(SearchPredicate),
}

/// A single JSON search predicate.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchPredicate {
    /// Public field name exposed by schema metadata.
    pub field: String,
    /// Operator to apply to the field.
    pub operator: SearchOperator,
    /// JSON value supplied by the client.
    #[serde(default)]
    pub value: JsonValue,
}

/// Operators supported by the JSON search API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum SearchOperator {
    /// Equality comparison.
    Equals,
    /// Inequality comparison.
    NotEquals,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Gte,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Lte,
    /// `IS NULL`.
    IsNull,
    /// `IS NOT NULL`.
    IsNotNull,
    /// Membership in a JSON array of values.
    In,
    /// Negated membership in a JSON array of values.
    NotIn,
    /// Inclusive range comparison with a two-element JSON array.
    Between,
    /// Negated inclusive range comparison with a two-element JSON array.
    NotBetween,
    /// Case-insensitive substring match with escaping.
    Contains,
    /// Negated case-insensitive substring match with escaping.
    NotContains,
    /// Case-insensitive prefix match with escaping.
    StartsWith,
    /// Negated case-insensitive prefix match with escaping.
    NotStartsWith,
    /// Case-insensitive suffix match with escaping.
    EndsWith,
    /// Negated case-insensitive suffix match with escaping.
    NotEndsWith,
    /// SQL `LIKE` match.
    Like,
    /// Negated SQL `LIKE` match.
    NotLike,
    /// SQL `ILIKE` match.
    ILike,
    /// Negated SQL `ILIKE` match.
    NotILike,
    /// Case-sensitive POSIX regular expression match.
    Regex,
    /// Negated case-sensitive POSIX regular expression match.
    NotRegex,
    /// Case-insensitive POSIX regular expression match.
    IRegex,
    /// Negated case-insensitive POSIX regular expression match.
    NotIRegex,
}

/// A client-supplied sort key.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchSort {
    /// Public field name exposed by schema metadata.
    pub field: String,
    /// Sort direction.
    pub dir: SortDirection,
}

/// Sort direction for [`SearchSort`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AndFilterWire {
    and: Vec<SearchFilter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrFilterWire {
    or: Vec<SearchFilter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NotFilterWire {
    not: Box<SearchFilter>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SearchFilterWire {
    And(AndFilterWire),
    Or(OrFilterWire),
    Not(NotFilterWire),
    Predicate(SearchPredicate),
}

impl<'de> Deserialize<'de> for SearchFilter {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = SearchFilterWire::deserialize(deserializer)?;
        Ok(match wire {
            SearchFilterWire::And(wire) => Self::And(wire.and),
            SearchFilterWire::Or(wire) => Self::Or(wire.or),
            SearchFilterWire::Not(wire) => Self::Not(wire.not),
            SearchFilterWire::Predicate(predicate) => Self::Predicate(predicate),
        })
    }
}
