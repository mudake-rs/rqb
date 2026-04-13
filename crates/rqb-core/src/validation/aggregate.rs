use std::collections::BTreeSet;

use crate::aggregate::Aggregate;
use crate::error::{Error, Result};
use crate::expr::Expr;
use crate::field::{FieldRef, ResolvedField};
use crate::types::FieldType;

use super::expr::validate_expr;
use super::resolve::resolve_field_in_scope;
use super::scope::{ExprContext, QueryScope};
use super::sort::validate_sort;
use super::{ValidatedAggregate, ValidatedExpr};

pub(super) fn validate_aggregate(
    scope: &QueryScope,
    aggregate: &Aggregate,
) -> Result<ValidatedAggregate> {
    Ok(match aggregate {
        Aggregate::Count { alias, filter } => ValidatedAggregate::Count {
            alias: alias.clone(),
            filter: validate_aggregate_filter(scope, filter)?,
        },
        Aggregate::CountField {
            field,
            alias,
            distinct,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            ValidatedAggregate::CountField {
                field,
                alias: alias.clone(),
                distinct: *distinct,
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
        Aggregate::Sum {
            field,
            alias,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            validate_numeric_aggregate_field("sum", &field)?;
            ValidatedAggregate::Sum {
                field,
                alias: alias.clone(),
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
        Aggregate::Avg {
            field,
            alias,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            validate_numeric_aggregate_field("avg", &field)?;
            ValidatedAggregate::Avg {
                field,
                alias: alias.clone(),
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
        Aggregate::Min {
            field,
            alias,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            validate_ordered_aggregate_field(&field)?;
            ValidatedAggregate::Min {
                field,
                alias: alias.clone(),
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
        Aggregate::Max {
            field,
            alias,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            validate_ordered_aggregate_field(&field)?;
            ValidatedAggregate::Max {
                field,
                alias: alias.clone(),
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
        Aggregate::JsonAgg {
            alias,
            fields,
            order_by,
            filter,
            default_empty,
        } => {
            let fields = fields
                .iter()
                .map(|field| resolve_aggregate_field(scope, field))
                .collect::<Result<Vec<_>>>()?;
            let order_by = order_by
                .as_ref()
                .map(|sort| validate_sort(scope, sort))
                .transpose()?;
            ValidatedAggregate::JsonAgg {
                alias: alias.clone(),
                fields,
                order_by,
                filter: validate_aggregate_filter(scope, filter)?,
                default_empty: *default_empty,
            }
        }
        Aggregate::ArrayAgg {
            field,
            alias,
            distinct,
            order_by,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            ValidatedAggregate::ArrayAgg {
                field,
                alias: alias.clone(),
                distinct: *distinct,
                order_by: order_by
                    .as_ref()
                    .map(|sort| validate_sort(scope, sort))
                    .transpose()?,
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
        Aggregate::StringAgg {
            field,
            separator,
            alias,
            order_by,
            filter,
        } => {
            let field = resolve_aggregate_field(scope, field)?;
            validate_string_aggregate_field(&field)?;
            ValidatedAggregate::StringAgg {
                field,
                separator: separator.clone(),
                alias: alias.clone(),
                order_by: order_by
                    .as_ref()
                    .map(|sort| validate_sort(scope, sort))
                    .transpose()?,
                filter: validate_aggregate_filter(scope, filter)?,
            }
        }
    })
}

pub(super) fn validate_aggregate_aliases(aggregates: &[ValidatedAggregate]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for aggregate in aggregates {
        let alias = aggregate.alias();
        if !seen.insert(alias.to_owned()) {
            return Err(Error::DuplicateAggregateAlias {
                alias: alias.to_owned(),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_grouped_selection(
    selected_fields: &[ResolvedField],
    group_by: &[ResolvedField],
    aggregates: &[ValidatedAggregate],
) -> Result<()> {
    if group_by.is_empty() || aggregates.is_empty() {
        return Ok(());
    }
    for field in selected_fields {
        if !group_by
            .iter()
            .any(|group| same_resolved_field(group, field))
        {
            return Err(Error::UngroupedField {
                field: field.display_name(),
            });
        }
    }
    Ok(())
}

fn validate_aggregate_filter(
    scope: &QueryScope,
    filter: &Option<Expr>,
) -> Result<Option<ValidatedExpr>> {
    filter
        .as_ref()
        .map(|filter| validate_expr(scope, filter, ExprContext::Filter))
        .transpose()
}

fn resolve_aggregate_field(scope: &QueryScope, field_ref: &FieldRef) -> Result<ResolvedField> {
    let field = resolve_field_in_scope(scope, field_ref)?;
    if field.is_json_path() {
        return Err(Error::NotSelectable {
            field: field.display_name(),
        });
    }
    if !field.caps.selectable {
        return Err(Error::NotSelectable {
            field: field.display_name(),
        });
    }
    Ok(field)
}

fn validate_numeric_aggregate_field(aggregate: &str, field: &ResolvedField) -> Result<()> {
    if field.ty.is_numeric() {
        return Ok(());
    }
    Err(Error::UnsupportedAggregateField {
        aggregate: aggregate.to_owned(),
        field: field.display_name(),
        field_type: field.ty.display_name().into_owned(),
    })
}

fn validate_ordered_aggregate_field(field: &ResolvedField) -> Result<()> {
    if field.caps.sortable {
        return Ok(());
    }
    Err(Error::NotSortable {
        field: field.display_name(),
    })
}

fn validate_string_aggregate_field(field: &ResolvedField) -> Result<()> {
    if field.ty.is_text() || matches!(field.ty, FieldType::Enum(_)) {
        return Ok(());
    }
    Err(Error::UnsupportedAggregateField {
        aggregate: "string_agg".to_owned(),
        field: field.display_name(),
        field_type: field.ty.display_name().into_owned(),
    })
}

fn same_resolved_field(left: &ResolvedField, right: &ResolvedField) -> bool {
    left.db_name == right.db_name
        && left.api_name == right.api_name
        && left.qualifier == right.qualifier
        && left.explicit_qualifier == right.explicit_qualifier
        && left.json_path == right.json_path
}
