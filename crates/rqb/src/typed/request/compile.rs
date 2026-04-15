use serde_json::Value as JsonValue;

use crate::typed::{
    BoolExpr, BoolOp, JsonKind, Meta, OrderDirection, OrderItem, Param, Select, Source, ValueExpr,
    expr::escape_like,
};
use crate::{Error, Result};

use super::value::{json_array, json_param};
use super::{
    SearchFilter, SearchOperator, SearchPredicate, SearchRequest, SearchSort, SortDirection,
};

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
                let op = self
                    .bool_op()
                    .expect("comparison branch must contain only comparison search operators");
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
