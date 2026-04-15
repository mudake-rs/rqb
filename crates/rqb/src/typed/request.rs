use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::typed::{
    BoolExpr, BoolOp, JsonKind, Meta, OrderDirection, OrderItem, Param, Select, Source, ValueExpr,
    expr::escape_like,
};
use crate::{Error, Result};

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

impl SearchRequest {
    pub fn merge_in(&self, mut select: Select) -> Result<Select> {
        let request_filter = self.filter_expr(&select.source)?;
        select.filter = match (select.filter, request_filter) {
            (Some(existing), Some(request)) => Some(BoolExpr::And(vec![existing, request])),
            (existing, None) => existing,
            (None, Some(request)) => Some(request),
        };
        self.apply_page_and_sort(select)
    }

    pub fn replace_in(&self, mut select: Select) -> Result<Select> {
        select.filter = self.filter_expr(&select.source)?;
        self.apply_page_and_sort(select)
    }

    fn filter_expr(&self, source: &Source) -> Result<Option<BoolExpr>> {
        self.filter
            .as_ref()
            .map(|filter| filter.to_expr(source))
            .transpose()
    }

    fn apply_page_and_sort(&self, mut select: Select) -> Result<Select> {
        select.order = self
            .sort
            .iter()
            .map(|sort| sort.to_order_item(&select.source))
            .collect::<Result<Vec<_>>>()?;
        select.limit = self.limit.map(|limit| Param::typed(i64::from(limit)));
        select.offset = self.offset.map(|offset| Param::typed(i64::from(offset)));
        Ok(select)
    }
}

impl SearchFilter {
    fn to_expr(&self, source: &Source) -> Result<BoolExpr> {
        match self {
            Self::And(filters) => {
                if filters.is_empty() {
                    return Err(Error::EmptySearchLogical { logical: "and" });
                }
                Ok(BoolExpr::And(
                    filters
                        .iter()
                        .map(|filter| filter.to_expr(source))
                        .collect::<Result<Vec<_>>>()?,
                ))
            }
            Self::Or(filters) => {
                if filters.is_empty() {
                    return Err(Error::EmptySearchLogical { logical: "or" });
                }
                Ok(BoolExpr::Or(
                    filters
                        .iter()
                        .map(|filter| filter.to_expr(source))
                        .collect::<Result<Vec<_>>>()?,
                ))
            }
            Self::Not(filter) => Ok(BoolExpr::Not(Box::new(filter.to_expr(source)?))),
            Self::Predicate(predicate) => predicate.to_expr(source),
        }
    }
}

impl SearchPredicate {
    fn to_expr(&self, source: &Source) -> Result<BoolExpr> {
        let meta = find_json_meta(source, &self.field)?;
        let Some(json) = meta.json else {
            return Err(Error::SearchFieldNotExposed {
                field: self.field.clone(),
            });
        };
        let field = ValueExpr::Field {
            meta,
            qualifier: source.explicit_alias().map(str::to_owned),
        };
        self.operator
            .to_expr(&self.field, meta, json, field, &self.value)
    }
}

impl SearchOperator {
    fn to_expr(
        self,
        field_name: &str,
        meta: Meta,
        json: JsonKind,
        field: ValueExpr,
        value: &JsonValue,
    ) -> Result<BoolExpr> {
        match self {
            Self::Equals | Self::NotEquals | Self::Gt | Self::Gte | Self::Lt | Self::Lte => {
                let op = self.bool_op().expect("comparison operator");
                validate_search_capability(
                    field_name,
                    meta,
                    self.as_name(),
                    op.requires_ordering(),
                )?;
                Ok(BoolExpr::Compare {
                    left: field,
                    op,
                    right: ValueExpr::Param(json_param(field_name, json, value)?),
                })
            }
            Self::IsNull | Self::IsNotNull => Ok(BoolExpr::IsNull {
                expr: field,
                negated: matches!(self, Self::IsNotNull),
            }),
            Self::In | Self::NotIn => {
                validate_search_capability(field_name, meta, self.as_name(), false)?;
                let values = json_array(field_name, value, "array")?
                    .iter()
                    .map(|value| json_param(field_name, json, value).map(ValueExpr::Param))
                    .collect::<Result<Vec<_>>>()?;
                if values.is_empty() {
                    return Ok(BoolExpr::Constant(matches!(self, Self::NotIn)));
                }
                Ok(BoolExpr::InList {
                    expr: field,
                    values,
                    negated: matches!(self, Self::NotIn),
                })
            }
            Self::Between | Self::NotBetween => {
                validate_search_capability(field_name, meta, self.as_name(), true)?;
                let values = json_array(field_name, value, "two-element array")?;
                let [low, high] = values.as_slice() else {
                    return Err(Error::InvalidSearchValue {
                        field: field_name.to_owned(),
                        expected: "two-element array",
                    });
                };
                Ok(BoolExpr::Between {
                    expr: field,
                    low: ValueExpr::Param(json_param(field_name, json, low)?),
                    high: ValueExpr::Param(json_param(field_name, json, high)?),
                    negated: matches!(self, Self::NotBetween),
                })
            }
            Self::Contains
            | Self::NotContains
            | Self::StartsWith
            | Self::NotStartsWith
            | Self::EndsWith
            | Self::NotEndsWith => {
                validate_search_like(field_name, meta, self.as_name())?;
                let value = value.as_str().ok_or_else(|| Error::InvalidSearchValue {
                    field: field_name.to_owned(),
                    expected: "string",
                })?;
                let (prefix, suffix) = match self {
                    Self::Contains | Self::NotContains => ("%", "%"),
                    Self::StartsWith | Self::NotStartsWith => ("", "%"),
                    Self::EndsWith | Self::NotEndsWith => ("%", ""),
                    _ => unreachable!(),
                };
                Ok(BoolExpr::Like {
                    expr: field,
                    pattern: ValueExpr::Param(Param::typed(format!(
                        "{prefix}{}{suffix}",
                        escape_like(value)
                    ))),
                    case_insensitive: true,
                    negated: matches!(
                        self,
                        Self::NotContains | Self::NotStartsWith | Self::NotEndsWith
                    ),
                    escape: true,
                })
            }
            Self::Like | Self::NotLike | Self::ILike | Self::NotILike => {
                validate_search_like(field_name, meta, self.as_name())?;
                let pattern = value
                    .as_str()
                    .map(|value| ValueExpr::Param(Param::typed(value.to_owned())))
                    .ok_or_else(|| Error::InvalidSearchValue {
                        field: field_name.to_owned(),
                        expected: "string",
                    })?;
                Ok(BoolExpr::Like {
                    expr: field,
                    pattern,
                    case_insensitive: matches!(self, Self::ILike | Self::NotILike),
                    negated: matches!(self, Self::NotLike | Self::NotILike),
                    escape: false,
                })
            }
            Self::Regex | Self::NotRegex | Self::IRegex | Self::NotIRegex => {
                validate_search_like(field_name, meta, self.as_name())?;
                let pattern = value
                    .as_str()
                    .map(|value| ValueExpr::Param(Param::typed(value.to_owned())))
                    .ok_or_else(|| Error::InvalidSearchValue {
                        field: field_name.to_owned(),
                        expected: "string",
                    })?;
                Ok(BoolExpr::Regex {
                    expr: field,
                    pattern,
                    case_insensitive: matches!(self, Self::IRegex | Self::NotIRegex),
                    negated: matches!(self, Self::NotRegex | Self::NotIRegex),
                })
            }
        }
    }

    const fn bool_op(self) -> Option<BoolOp> {
        match self {
            Self::Equals => Some(BoolOp::Eq),
            Self::NotEquals => Some(BoolOp::Ne),
            Self::Gt => Some(BoolOp::Gt),
            Self::Gte => Some(BoolOp::Gte),
            Self::Lt => Some(BoolOp::Lt),
            Self::Lte => Some(BoolOp::Lte),
            Self::IsNull
            | Self::IsNotNull
            | Self::In
            | Self::NotIn
            | Self::Between
            | Self::NotBetween
            | Self::Contains
            | Self::NotContains
            | Self::StartsWith
            | Self::NotStartsWith
            | Self::EndsWith
            | Self::NotEndsWith
            | Self::Like
            | Self::NotLike
            | Self::ILike
            | Self::NotILike
            | Self::Regex
            | Self::NotRegex
            | Self::IRegex
            | Self::NotIRegex => None,
        }
    }

    const fn as_name(self) -> &'static str {
        match self {
            Self::Equals => "equals",
            Self::NotEquals => "notEquals",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::IsNull => "isNull",
            Self::IsNotNull => "isNotNull",
            Self::In => "in",
            Self::NotIn => "notIn",
            Self::Between => "between",
            Self::NotBetween => "notBetween",
            Self::Contains => "contains",
            Self::NotContains => "notContains",
            Self::StartsWith => "startsWith",
            Self::NotStartsWith => "notStartsWith",
            Self::EndsWith => "endsWith",
            Self::NotEndsWith => "notEndsWith",
            Self::Like => "like",
            Self::NotLike => "notLike",
            Self::ILike => "iLike",
            Self::NotILike => "notILike",
            Self::Regex => "regex",
            Self::NotRegex => "notRegex",
            Self::IRegex => "iRegex",
            Self::NotIRegex => "notIRegex",
        }
    }
}

impl SearchSort {
    fn to_order_item(&self, source: &Source) -> Result<OrderItem> {
        let meta = find_json_meta(source, &self.field)?;
        if !meta.ops.ordering {
            return Err(Error::InvalidTypedSort {
                field: self.field.clone(),
            });
        }
        Ok(OrderItem {
            expr: ValueExpr::Field {
                meta,
                qualifier: source.explicit_alias().map(str::to_owned),
            },
            direction: self.dir.into(),
        })
    }
}

impl From<SortDirection> for OrderDirection {
    fn from(direction: SortDirection) -> Self {
        match direction {
            SortDirection::Asc => Self::Asc,
            SortDirection::Desc => Self::Desc,
        }
    }
}

impl Select {
    pub fn request(self, request: SearchRequest) -> Result<Self> {
        request.merge_in(self)
    }

    pub fn replace_request(self, request: SearchRequest) -> Result<Self> {
        request.replace_in(self)
    }
}

fn find_json_meta(source: &Source, field: &str) -> Result<Meta> {
    let mut found = None;
    source.for_each_field(|meta| {
        if meta.api == field {
            found = Some(*meta);
        }
    });
    let Some(meta) = found else {
        return Err(Error::InvalidSearchField {
            field: field.to_owned(),
        });
    };
    if meta.json.is_none() {
        return Err(Error::SearchFieldNotExposed {
            field: field.to_owned(),
        });
    }
    Ok(meta)
}

fn validate_search_capability(
    field: &str,
    meta: Meta,
    operator: &'static str,
    requires_ordering: bool,
) -> Result<()> {
    let supported = if requires_ordering {
        meta.ops.ordering
    } else {
        meta.ops.equality
    };
    if supported {
        return Ok(());
    }
    Err(Error::InvalidSearchOperator {
        field: field.to_owned(),
        operator: operator.to_owned(),
    })
}

fn validate_search_like(field: &str, meta: Meta, operator: &'static str) -> Result<()> {
    if matches!(meta.pg, "text" | "varchar" | "bpchar" | "citext") {
        return Ok(());
    }
    Err(Error::InvalidSearchOperator {
        field: field.to_owned(),
        operator: operator.to_owned(),
    })
}

fn json_array<'a>(
    field: &str,
    value: &'a JsonValue,
    expected: &'static str,
) -> Result<&'a Vec<JsonValue>> {
    value.as_array().ok_or_else(|| Error::InvalidSearchValue {
        field: field.to_owned(),
        expected,
    })
}

fn json_param(field: &str, kind: JsonKind, value: &JsonValue) -> Result<Param> {
    match kind {
        JsonKind::Text => value
            .as_str()
            .map(|value| Param::typed(value.to_owned()))
            .ok_or_else(|| invalid_value(field, "string")),
        JsonKind::Bool => value
            .as_bool()
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "boolean")),
        JsonKind::Integer => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "32-bit integer")),
        JsonKind::BigInt => value
            .as_i64()
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "64-bit integer")),
        JsonKind::Float => value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "finite number")),
        JsonKind::NumericString => value
            .as_str()
            .and_then(|value| value.parse::<sqlx::types::BigDecimal>().ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "decimal string")),
        JsonKind::Uuid => value
            .as_str()
            .and_then(|value| value.parse::<uuid::Uuid>().ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "UUID string")),
        JsonKind::Date => value
            .as_str()
            .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "date string")),
        JsonKind::Time => value
            .as_str()
            .and_then(|value| chrono::NaiveTime::parse_from_str(value, "%H:%M:%S%.f").ok())
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "time string")),
        JsonKind::Timestamp => value
            .as_str()
            .and_then(parse_naive_datetime)
            .map(Param::typed)
            .ok_or_else(|| invalid_value(field, "timestamp string")),
        JsonKind::Timestamptz => value
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| Param::typed(value.with_timezone(&chrono::Utc)))
            .ok_or_else(|| invalid_value(field, "RFC3339 timestamp string")),
        JsonKind::Jsonb => Ok(Param::typed(value.clone())),
    }
}

fn parse_naive_datetime(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f"))
        .ok()
}

fn invalid_value(field: &str, expected: &'static str) -> Error {
    Error::InvalidSearchValue {
        field: field.to_owned(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SearchRequest;
    use crate::typed::{Field, JsonKind, Meta, OpSet, Source};

    static ID_META: Meta = Meta::new("id", "id", "int4")
        .ops(OpSet::ordered())
        .json(JsonKind::Integer);
    static STATUS_META: Meta = Meta::new("status", "status", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    static ACTIVE_META: Meta = Meta::new("active", "active", "bool")
        .ops(OpSet::equality())
        .json(JsonKind::Bool);
    static INTERNAL_META: Meta = Meta::new("internal", "internal", "text").ops(OpSet::ordered());
    static FIELDS: [&Meta; 4] = [&ID_META, &STATUS_META, &ACTIVE_META, &INTERNAL_META];
    const ID: Field<i32> = Field::new(&ID_META);

    fn source() -> Source {
        Source::Table {
            name: "public.orders",
            alias: None,
            fields: &FIELDS,
        }
    }

    #[test]
    fn search_request_merges_filter_and_applies_sort_limit_offset() {
        let request: SearchRequest = serde_json::from_value(json!({
            "filter": {
                "and": [
                    { "field": "status", "operator": "equals", "value": "paid" },
                    { "field": "id", "operator": "gte", "value": 100 }
                ]
            },
            "sort": [{ "field": "id", "dir": "desc" }],
            "limit": 20,
            "offset": 40
        }))
        .unwrap();

        let built = crate::typed::select(source())
            .filter(ID.gt(10))
            .request(request)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (\"id\" > $1 AND (\"status\" = $2 AND \"id\" >= $3)) ORDER BY \"id\" DESC LIMIT $4 OFFSET $5"
        );
        assert_eq!(built.params.len(), 5);
    }

    #[test]
    fn search_request_qualifies_fields_when_root_source_is_aliased() {
        let request: SearchRequest = serde_json::from_value(json!({
            "filter": { "field": "status", "operator": "equals", "value": "paid" },
            "sort": [{ "field": "id", "dir": "asc" }]
        }))
        .unwrap();

        let built = crate::typed::select(source().alias("o"))
            .request(request)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"o\".\"id\", \"o\".\"status\", \"o\".\"active\", \"o\".\"internal\" FROM \"public\".\"orders\" AS \"o\" WHERE \"o\".\"status\" = $1 ORDER BY \"o\".\"id\" ASC"
        );
    }

    #[test]
    fn search_request_supports_null_in_between_and_like() {
        let request: SearchRequest = serde_json::from_value(json!({
            "filter": {
                "and": [
                    { "field": "status", "operator": "isNotNull" },
                    { "field": "status", "operator": "in", "value": ["paid", "open"] },
                    { "field": "id", "operator": "between", "value": [10, 20] },
                    { "field": "status", "operator": "contains", "value": "50%_match" },
                    { "field": "status", "operator": "regex", "value": "^p" },
                    { "field": "status", "operator": "iRegex", "value": "^paid" },
                    { "field": "status", "operator": "iLike", "value": "p%" }
                ]
            }
        }))
        .unwrap();

        let built = crate::typed::select(source())
            .request(request)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE (\"status\" IS NOT NULL AND \"status\" IN ($1, $2) AND \"id\" BETWEEN $3 AND $4 AND \"status\" ILIKE $5 ESCAPE '\\' AND \"status\" ~ $6 AND \"status\" ~* $7 AND \"status\" ILIKE $8)"
        );
        assert_eq!(built.params.len(), 8);
    }

    #[test]
    fn replace_request_replaces_existing_filter() {
        let request: SearchRequest = serde_json::from_value(json!({
            "filter": { "field": "status", "operator": "equals", "value": "paid" }
        }))
        .unwrap();

        let built = crate::typed::select(source())
            .filter(ID.gt(10))
            .replace_request(request)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            built.sql,
            "SELECT \"id\", \"status\", \"active\", \"internal\" FROM \"public\".\"orders\" WHERE \"status\" = $1"
        );
    }

    #[test]
    fn hidden_search_field_is_rejected() {
        let request: SearchRequest = serde_json::from_value(json!({
            "filter": { "field": "internal", "operator": "equals", "value": "x" }
        }))
        .unwrap();

        let err = crate::typed::select(source()).request(request).unwrap_err();

        assert!(matches!(
            err,
            crate::Error::SearchFieldNotExposed { field } if field == "internal"
        ));
    }

    #[test]
    fn invalid_json_value_is_rejected_before_rendering() {
        let request: SearchRequest = serde_json::from_value(json!({
            "filter": { "field": "active", "operator": "equals", "value": "yes" }
        }))
        .unwrap();

        let err = crate::typed::select(source()).request(request).unwrap_err();

        assert!(matches!(
            err,
            crate::Error::InvalidSearchValue { field, expected: "boolean" }
                if field == "active"
        ));
    }

    #[test]
    fn json_request_does_not_accept_projection_fields() {
        let err = serde_json::from_value::<SearchRequest>(json!({
            "fields": ["id"]
        }))
        .unwrap_err();

        assert!(err.to_string().contains("unknown field"));
    }
}
