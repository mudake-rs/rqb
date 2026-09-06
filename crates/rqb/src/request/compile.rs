use serde_json::Value as JsonValue;

use crate::{
    BoolExpr, BoolOp, JsonKind, Meta, OrderDirection, OrderItem, Param, RowLimit, Select, Source,
    ValueExpr, expr::escaped_like_pattern,
};
use crate::{Error, Result};

use super::value::{json_array, json_param};
use super::{
    SearchFilter, SearchOperator, SearchPredicate, SearchRequest, SearchSort, SortDirection,
};

const MAX_SEARCH_PATTERN_CHARS: usize = 1024;

impl Select {
    /// AND-composes a JSON filter without replacing server-owned sorting or pagination.
    ///
    /// The same root-field exposure and operator checks as [`Self::apply_search`] apply.
    pub fn apply_filter(self, filter: SearchFilter) -> Result<Self> {
        let predicate = filter.to_expr(&SearchMetaLookup::new(&self.source))?;
        Ok(self.filter(predicate))
    }
}

impl SearchRequest {
    /// Merges this request into an existing select, preserving server filters.
    ///
    /// The request filter is AND-composed with the select's existing filter.
    /// Sort, limit, and offset are client-owned request clauses, so they
    /// replace the builder's current values for those clauses.
    pub(crate) fn merge_in(&self, mut select: Select) -> Result<Select> {
        let lookup = SearchMetaLookup::new(&select.source);
        let request_filter = self.filter_expr(&lookup)?;
        let order = self.order_items(&lookup)?;
        let row_limit = self
            .limit
            .map(|limit| RowLimit::Limit(Param::typed(i64::from(limit))));
        let offset = self.offset.map(|offset| Param::typed(i64::from(offset)));
        select.filter = match (select.filter, request_filter) {
            (Some(existing), Some(request)) => Some(BoolExpr::and_pair(existing, request)),
            (existing, None) => existing,
            (None, Some(request)) => Some(request),
        };
        select.order = order;
        select.row_limit = row_limit;
        select.offset = offset;
        Ok(select)
    }

    fn filter_expr(&self, lookup: &SearchMetaLookup) -> Result<Option<BoolExpr>> {
        self.filter
            .as_ref()
            .map(|filter| filter.to_expr(lookup))
            .transpose()
    }

    fn order_items(&self, lookup: &SearchMetaLookup<'_>) -> Result<Vec<OrderItem>> {
        self.sort
            .iter()
            .map(|sort| sort.to_order_item(lookup))
            .collect()
    }
}

impl SearchFilter {
    fn to_expr(&self, lookup: &SearchMetaLookup<'_>) -> Result<BoolExpr> {
        match self {
            Self::And(filters) => {
                if filters.is_empty() {
                    return Err(Error::EmptySearchLogical { logical: "and" });
                }
                Ok(BoolExpr::And(
                    filters
                        .iter()
                        .map(|filter| filter.to_expr(lookup))
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
                        .map(|filter| filter.to_expr(lookup))
                        .collect::<Result<Vec<_>>>()?,
                ))
            }
            Self::Not(filter) => Ok(BoolExpr::Not(Box::new(filter.to_expr(lookup)?))),
            Self::Predicate(predicate) => predicate.to_expr(lookup),
        }
    }
}

impl SearchPredicate {
    fn to_expr(&self, lookup: &SearchMetaLookup<'_>) -> Result<BoolExpr> {
        let (meta, json) = lookup.json_meta(&self.field)?;
        let field = lookup.field_expr(meta);
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
            Self::Equals => {
                comparison_expr(field_name, meta, "equals", BoolOp::Eq, field, json, value)
            }
            Self::NotEquals => comparison_expr(
                field_name,
                meta,
                "notEquals",
                BoolOp::Ne,
                field,
                json,
                value,
            ),
            Self::Gt => comparison_expr(field_name, meta, "gt", BoolOp::Gt, field, json, value),
            Self::Gte => comparison_expr(field_name, meta, "gte", BoolOp::Gte, field, json, value),
            Self::Lt => comparison_expr(field_name, meta, "lt", BoolOp::Lt, field, json, value),
            Self::Lte => comparison_expr(field_name, meta, "lte", BoolOp::Lte, field, json, value),
            Self::IsNull | Self::IsNotNull => {
                validate_search_capability(field_name, meta, self.as_name(), false)?;
                Ok(BoolExpr::is_null_expr(
                    field,
                    matches!(self, Self::IsNotNull),
                ))
            }
            Self::In | Self::NotIn => {
                validate_search_capability(field_name, meta, self.as_name(), false)?;
                let values = json_array(field_name, value, "array")?
                    .iter()
                    .map(|value| json_param(field_name, json, value).map(ValueExpr::Param))
                    .collect::<Result<Vec<_>>>()?;
                if values.is_empty() {
                    return Ok(BoolExpr::Constant(matches!(self, Self::NotIn)));
                }
                Ok(BoolExpr::in_list(
                    field,
                    values,
                    matches!(self, Self::NotIn),
                ))
            }
            Self::Between | Self::NotBetween => {
                validate_search_capability(field_name, meta, self.as_name(), true)?;
                let values = json_array(field_name, value, "two-element array")?;
                let [low, high] = values.as_slice() else {
                    return Err(Error::invalid_search_value(field_name, "two-element array"));
                };
                Ok(BoolExpr::between(
                    field,
                    ValueExpr::Param(json_param(field_name, json, low)?),
                    ValueExpr::Param(json_param(field_name, json, high)?),
                    matches!(self, Self::NotBetween),
                ))
            }
            Self::Contains
            | Self::NotContains
            | Self::StartsWith
            | Self::NotStartsWith
            | Self::EndsWith
            | Self::NotEndsWith => {
                validate_search_pattern(field_name, meta, json, self.as_name())?;
                let value = search_pattern_value(field_name, value)?;
                let (prefix, suffix) = match self {
                    Self::Contains | Self::NotContains => ("%", "%"),
                    Self::StartsWith | Self::NotStartsWith => ("", "%"),
                    Self::EndsWith | Self::NotEndsWith => ("%", ""),
                    _ => unreachable!(),
                };
                Ok(BoolExpr::like(
                    field,
                    ValueExpr::Param(Param::typed(escaped_like_pattern(value, prefix, suffix))),
                    true,
                    matches!(
                        self,
                        Self::NotContains | Self::NotStartsWith | Self::NotEndsWith
                    ),
                    true,
                ))
            }
            Self::Like | Self::NotLike | Self::ILike | Self::NotILike => {
                validate_search_pattern(field_name, meta, json, self.as_name())?;
                let pattern = ValueExpr::Param(Param::typed(
                    search_pattern_value(field_name, value)?.to_owned(),
                ));
                Ok(BoolExpr::like(
                    field,
                    pattern,
                    matches!(self, Self::ILike | Self::NotILike),
                    matches!(self, Self::NotLike | Self::NotILike),
                    false,
                ))
            }
            Self::Regex | Self::NotRegex | Self::IRegex | Self::NotIRegex => {
                validate_search_pattern(field_name, meta, json, self.as_name())?;
                let pattern = ValueExpr::Param(Param::typed(
                    search_pattern_value(field_name, value)?.to_owned(),
                ));
                Ok(BoolExpr::regex(
                    field,
                    pattern,
                    matches!(self, Self::IRegex | Self::NotIRegex),
                    matches!(self, Self::NotRegex | Self::NotIRegex),
                ))
            }
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

fn comparison_expr(
    field_name: &str,
    meta: Meta,
    operator_name: &'static str,
    op: BoolOp,
    field: ValueExpr,
    json: JsonKind,
    value: &JsonValue,
) -> Result<BoolExpr> {
    validate_search_capability(field_name, meta, operator_name, op.requires_ordering())?;
    Ok(BoolExpr::compare(
        field,
        op,
        ValueExpr::Param(json_param(field_name, json, value)?),
    ))
}

impl SearchSort {
    fn to_order_item(&self, lookup: &SearchMetaLookup<'_>) -> Result<OrderItem> {
        let (meta, _) = lookup.json_meta(&self.field)?;
        if !meta.ops.ordering {
            return Err(Error::InvalidSort {
                field: self.field.clone(),
            });
        }
        Ok(OrderItem {
            expr: lookup.field_expr(meta),
            direction: self.dir.into(),
            nulls: None,
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
    /// Applies a JSON search request.
    ///
    /// The request filter is AND-composed with existing server filters. Sort,
    /// limit, and offset are request-controlled clauses, so request values
    /// replace any existing builder values for those clauses.
    ///
    /// Search operators are gated by field metadata: equality and null tests
    /// require equality capability, sort requires ordering capability, and
    /// LIKE/regex-style pattern operators require text-pattern capability.
    #[inline]
    pub fn apply_search(self, request: SearchRequest) -> Result<Self> {
        request.merge_in(self)
    }
}

struct SearchMetaLookup<'a> {
    source: &'a Source,
    qualifier: Option<&'a str>,
}

impl<'a> SearchMetaLookup<'a> {
    fn new(source: &'a Source) -> Self {
        Self {
            source,
            qualifier: source.explicit_alias(),
        }
    }

    fn json_meta(&self, field: &str) -> Result<(Meta, JsonKind)> {
        let Some(meta) = self.source.field_by_api(field) else {
            return Err(Error::InvalidSearchField {
                field: field.to_owned(),
            });
        };
        let Some(json) = meta.json else {
            return Err(Error::SearchFieldNotExposed {
                field: field.to_owned(),
            });
        };
        Ok((meta, json))
    }

    fn field_expr(&self, meta: Meta) -> ValueExpr {
        ValueExpr::field(meta, self.qualifier.map(str::to_owned))
    }
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
    Err(Error::invalid_search_operator(field, operator))
}

fn validate_search_pattern(
    field: &str,
    meta: Meta,
    json: JsonKind,
    operator: &'static str,
) -> Result<()> {
    if json == JsonKind::Text && meta.ops.pattern {
        return Ok(());
    }
    Err(Error::invalid_search_operator(field, operator))
}

fn search_pattern_value<'a>(field: &str, value: &'a JsonValue) -> Result<&'a str> {
    let value = value
        .as_str()
        .ok_or_else(|| Error::invalid_search_value(field, "string"))?;
    if value.chars().take(MAX_SEARCH_PATTERN_CHARS + 1).count() <= MAX_SEARCH_PATTERN_CHARS {
        return Ok(value);
    }
    Err(Error::invalid_search_value(
        field,
        "string up to 1024 characters",
    ))
}
