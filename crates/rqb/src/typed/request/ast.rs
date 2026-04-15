use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    #[serde(default)]
    pub filter: Option<SearchFilter>,
    #[serde(default)]
    pub sort: Vec<SearchSort>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

#[derive(Clone, Debug)]
pub enum SearchFilter {
    And(Vec<SearchFilter>),
    Or(Vec<SearchFilter>),
    Not(Box<SearchFilter>),
    Predicate(SearchPredicate),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchPredicate {
    pub field: String,
    pub operator: SearchOperator,
    #[serde(default)]
    pub value: JsonValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchOperator {
    Equals,
    NotEquals,
    Gt,
    Gte,
    Lt,
    Lte,
    IsNull,
    IsNotNull,
    In,
    NotIn,
    Between,
    NotBetween,
    Contains,
    NotContains,
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
    Like,
    NotLike,
    ILike,
    NotILike,
    Regex,
    NotRegex,
    IRegex,
    NotIRegex,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchSort {
    pub field: String,
    pub dir: SortDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SearchFilterWire {
    And { and: Vec<SearchFilter> },
    Or { or: Vec<SearchFilter> },
    Not { not: Box<SearchFilter> },
    Predicate(SearchPredicate),
}

impl<'de> Deserialize<'de> for SearchFilter {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = SearchFilterWire::deserialize(deserializer)?;
        Ok(match wire {
            SearchFilterWire::And { and } => Self::And(and),
            SearchFilterWire::Or { or } => Self::Or(or),
            SearchFilterWire::Not { not } => Self::Not(not),
            SearchFilterWire::Predicate(predicate) => Self::Predicate(predicate),
        })
    }
}
