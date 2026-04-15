use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::typed::{
    BoolExpr, BoolOp, JsonKind, Meta, OrderDirection, OrderItem, Param, Select, Source, ValueExpr,
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
        let op = self.operator.bool_op();
        let supported = if op.requires_ordering() {
            meta.ops.ordering
        } else {
            meta.ops.equality
        };
        if !supported {
            return Err(Error::InvalidSearchOperator {
                field: self.field.clone(),
                operator: self.operator.as_name().to_owned(),
            });
        }
        let Some(json) = meta.json else {
            return Err(Error::SearchFieldNotExposed {
                field: self.field.clone(),
            });
        };
        Ok(BoolExpr::Compare {
            left: ValueExpr::Field(meta),
            op,
            right: ValueExpr::Param(json_param(&self.field, json, &self.value)?),
        })
    }
}

impl SearchOperator {
    const fn bool_op(self) -> BoolOp {
        match self {
            Self::Equals => BoolOp::Eq,
            Self::NotEquals => BoolOp::Ne,
            Self::Gt => BoolOp::Gt,
            Self::Gte => BoolOp::Gte,
            Self::Lt => BoolOp::Lt,
            Self::Lte => BoolOp::Lte,
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
            expr: ValueExpr::Field(meta),
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
