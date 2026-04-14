use crate::expr::{Expr, Sort};
use crate::field::{FieldRef, ResolvedField};
use crate::types::FieldType;

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum Aggregate {
    Count {
        alias: String,
        filter: Option<Expr>,
    },
    CountField {
        field: FieldRef,
        alias: String,
        distinct: bool,
        filter: Option<Expr>,
    },
    Sum {
        field: FieldRef,
        alias: String,
        filter: Option<Expr>,
    },
    Avg {
        field: FieldRef,
        alias: String,
        filter: Option<Expr>,
    },
    Min {
        field: FieldRef,
        alias: String,
        filter: Option<Expr>,
    },
    Max {
        field: FieldRef,
        alias: String,
        filter: Option<Expr>,
    },
    JsonAgg {
        alias: String,
        fields: Vec<FieldRef>,
        order_by: Option<Sort>,
        filter: Option<Expr>,
        default_empty: bool,
    },
    ArrayAgg {
        field: FieldRef,
        alias: String,
        distinct: bool,
        order_by: Option<Sort>,
        filter: Option<Expr>,
    },
    StringAgg {
        field: FieldRef,
        separator: String,
        alias: String,
        order_by: Option<Sort>,
        filter: Option<Expr>,
    },
}

impl Aggregate {
    pub fn alias(&self) -> &str {
        match self {
            Self::Count { alias, .. }
            | Self::CountField { alias, .. }
            | Self::Sum { alias, .. }
            | Self::Avg { alias, .. }
            | Self::Min { alias, .. }
            | Self::Max { alias, .. }
            | Self::JsonAgg { alias, .. }
            | Self::ArrayAgg { alias, .. }
            | Self::StringAgg { alias, .. } => alias,
        }
    }

    pub fn order_by(mut self, sort: impl Into<Sort>) -> Self {
        match &mut self {
            Self::JsonAgg { order_by, .. }
            | Self::ArrayAgg { order_by, .. }
            | Self::StringAgg { order_by, .. } => *order_by = Some(sort.into()),
            _ => {}
        }
        self
    }

    pub fn filter(mut self, expr: impl Into<Expr>) -> Self {
        self.set_filter(expr.into());
        self
    }

    pub(crate) fn set_filter(&mut self, expr: Expr) {
        match self {
            Self::Count { filter, .. }
            | Self::CountField { filter, .. }
            | Self::Sum { filter, .. }
            | Self::Avg { filter, .. }
            | Self::Min { filter, .. }
            | Self::Max { filter, .. }
            | Self::JsonAgg { filter, .. }
            | Self::ArrayAgg { filter, .. }
            | Self::StringAgg { filter, .. } => *filter = Some(expr),
        }
    }

    pub fn distinct(mut self) -> Self {
        if let Self::ArrayAgg { distinct, .. } = &mut self {
            *distinct = true;
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum AggregateType {
    Count,
    Sum,
    Avg,
    Min(FieldType),
    Max(FieldType),
    Json,
    String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum SelectColumn {
    Field(ResolvedField),
    Aggregate { alias: String, ty: AggregateType },
    Expression { alias: String, ty: FieldType },
}

impl SelectColumn {
    pub fn alias(&self) -> String {
        match self {
            Self::Field(field) => field.output_alias(),
            Self::Aggregate { alias, .. } | Self::Expression { alias, .. } => alias.clone(),
        }
    }

    pub fn ty(&self) -> FieldType {
        match self {
            Self::Field(field) => field.ty,
            Self::Aggregate { ty, .. } => ty.field_type(),
            Self::Expression { ty, .. } => *ty,
        }
    }
}

impl AggregateType {
    pub fn field_type(&self) -> FieldType {
        match self {
            Self::Count => FieldType::BigInt,
            Self::Sum | Self::Avg => FieldType::Float,
            Self::Min(ty) | Self::Max(ty) => *ty,
            Self::Json => FieldType::Jsonb,
            Self::String => FieldType::Text,
        }
    }
}

pub fn count(alias: impl Into<String>) -> Aggregate {
    Aggregate::Count {
        alias: alias.into(),
        filter: None,
    }
}

pub fn count_field(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::CountField {
        field: field.into(),
        alias: alias.into(),
        distinct: false,
        filter: None,
    }
}

pub fn count_distinct(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::CountField {
        field: field.into(),
        alias: alias.into(),
        distinct: true,
        filter: None,
    }
}

pub fn sum(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::Sum {
        field: field.into(),
        alias: alias.into(),
        filter: None,
    }
}

pub fn avg(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::Avg {
        field: field.into(),
        alias: alias.into(),
        filter: None,
    }
}

pub fn min_agg(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    min(field, alias)
}

pub fn max_agg(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    max(field, alias)
}

pub fn min(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::Min {
        field: field.into(),
        alias: alias.into(),
        filter: None,
    }
}

pub fn max(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::Max {
        field: field.into(),
        alias: alias.into(),
        filter: None,
    }
}

pub fn array_agg(field: impl Into<FieldRef>, alias: impl Into<String>) -> Aggregate {
    Aggregate::ArrayAgg {
        field: field.into(),
        alias: alias.into(),
        distinct: false,
        order_by: None,
        filter: None,
    }
}

pub fn json_agg<I, F>(alias: impl Into<String>, fields: I) -> Aggregate
where
    I: IntoIterator<Item = F>,
    F: Into<FieldRef>,
{
    json_agg_with_default(alias, fields, true)
}

pub fn json_agg_nullable<I, F>(alias: impl Into<String>, fields: I) -> Aggregate
where
    I: IntoIterator<Item = F>,
    F: Into<FieldRef>,
{
    json_agg_with_default(alias, fields, false)
}

pub(crate) fn json_agg_with_default<I, F>(
    alias: impl Into<String>,
    fields: I,
    default_empty: bool,
) -> Aggregate
where
    I: IntoIterator<Item = F>,
    F: Into<FieldRef>,
{
    Aggregate::JsonAgg {
        alias: alias.into(),
        fields: fields.into_iter().map(Into::into).collect(),
        order_by: None,
        filter: None,
        default_empty,
    }
}

pub fn string_agg(
    field: impl Into<FieldRef>,
    separator: impl Into<String>,
    alias: impl Into<String>,
) -> Aggregate {
    Aggregate::StringAgg {
        field: field.into(),
        separator: separator.into(),
        alias: alias.into(),
        order_by: None,
        filter: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::field;

    #[test]
    fn json_agg_function_builds_filterable_aggregate() {
        let aggregate = json_agg("orders", [field("o.id"), field("o.status")])
            .filter(field("o.id").is_not_null())
            .order_by(field("o.createdAt").desc());

        let Aggregate::JsonAgg {
            alias,
            fields,
            order_by,
            filter,
            default_empty,
        } = aggregate
        else {
            panic!("expected JsonAgg");
        };

        assert_eq!(alias, "orders");
        assert_eq!(fields.len(), 2);
        assert!(order_by.is_some());
        assert!(filter.is_some());
        assert!(default_empty);
    }

    #[test]
    fn json_agg_nullable_function_preserves_sql_null_contract() {
        let Aggregate::JsonAgg { default_empty, .. } = json_agg_nullable("orders", [field("o.id")])
        else {
            panic!("expected JsonAgg");
        };

        assert!(!default_empty);
    }
}
