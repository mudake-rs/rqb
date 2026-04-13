use std::collections::BTreeSet;

use crate::aggregate::{Aggregate, SelectColumn};
use crate::dataset::{CteBody, Dataset};
use crate::error::{Error, Result};
use crate::expr::{Expr, LogicalOp, Sort};
use crate::field::{FieldRef, ResolvedField};
use crate::request::SelectQuery;
use crate::types::FieldType;

use super::operators::{count_raw_placeholders, validate_operator};
use super::resolve::{default_qualifier, resolve_field_in_scope, resolved_from_field};
use super::scope::{ExprContext, QueryScope};
use super::value_check::validate_column_operator;
use super::{
    ValidatedAggregate, ValidatedCte, ValidatedCteBody, ValidatedExpr, ValidatedJoin,
    ValidatedSelect, ValidatedSort,
};

impl ValidatedSelect {
    pub fn new(query: SelectQuery) -> Result<Self> {
        Self::new_with_outer_datasets(query, &[])
    }

    pub fn new_with_outer_datasets(query: SelectQuery, outer_datasets: &[Dataset]) -> Result<Self> {
        if let Some(error) = query.builder_errors.first() {
            return Err(error.clone());
        }
        let scope = QueryScope::new_with_outer(&query, outer_datasets)?;
        let ctes = validate_ctes(&query)?;
        let joins = validate_joins(&scope, &query)?;

        let limit_explicit = query.request.limit.is_some();
        let offset_explicit = query.request.offset.is_some();
        let limit = query.request.limit.unwrap_or(query.dataset.default_limit);
        if limit > query.dataset.max_limit {
            return Err(Error::LimitExceeded {
                requested: limit,
                max: query.dataset.max_limit,
            });
        }

        let mut selected_fields = resolve_selection(&scope, &query.request.fields)?;
        apply_root_output_aliases(&query, &mut selected_fields);
        let distinct_on = resolve_distinct_on(&scope, &query.distinct_on)?;
        let aggregates = query
            .aggregates
            .iter()
            .map(|aggregate| validate_aggregate(&scope, aggregate))
            .collect::<Result<Vec<_>>>()?;
        let group_by = if !query.group_by.is_empty() {
            resolve_group_by(&scope, &query.group_by)?
        } else if !aggregates.is_empty() {
            selected_fields.clone()
        } else {
            Vec::new()
        };
        validate_aggregate_aliases(&aggregates)?;
        validate_grouped_selection(&selected_fields, &group_by, &aggregates)?;
        let columns = select_columns(&selected_fields, &aggregates);
        let sort = query
            .request
            .sort
            .iter()
            .map(|sort| validate_sort(&scope, sort))
            .collect::<Result<Vec<_>>>()?;

        let filter = query
            .request
            .query
            .as_ref()
            .map(|expr| validate_expr(&scope, expr, ExprContext::Filter))
            .transpose()?;
        let having = query
            .having
            .as_ref()
            .map(|expr| validate_expr(&scope, expr, ExprContext::Having))
            .transpose()?;

        Ok(Self {
            offset: query.request.offset.unwrap_or(0),
            limit,
            limit_explicit,
            offset_explicit,
            dataset: query.dataset.clone(),
            cacheable: query.cacheable,
            distinct: query.distinct,
            lock: query.lock,
            ctes,
            joins,
            selected_fields,
            distinct_on,
            group_by,
            aggregates,
            columns,
            filter,
            having,
            sort,
        })
    }
}

fn apply_root_output_aliases(query: &SelectQuery, fields: &mut [ResolvedField]) {
    if query.joins.is_empty() {
        return;
    }
    let root_qualifier = query.dataset.sql_qualifier();
    for field in fields {
        if field.alias.is_none() && field.qualifier.as_deref() == Some(root_qualifier) {
            field.alias = Some(field.api_name.clone());
        }
    }
}

fn resolve_distinct_on(scope: &QueryScope, fields: &[FieldRef]) -> Result<Vec<ResolvedField>> {
    fields
        .iter()
        .map(|field| resolve_field_in_scope(scope, field))
        .collect()
}

fn validate_ctes(query: &SelectQuery) -> Result<Vec<ValidatedCte>> {
    query
        .ctes
        .iter()
        .map(|cte| {
            let body = match &cte.body {
                CteBody::Raw(raw) => {
                    let placeholders = count_raw_placeholders(&raw.sql);
                    if placeholders != raw.binds.len() {
                        return Err(Error::RawBindMismatch {
                            placeholders,
                            binds: raw.binds.len(),
                        });
                    }
                    ValidatedCteBody::Raw(raw.clone())
                }
                CteBody::Select(select) => {
                    ValidatedCteBody::Select(Box::new(ValidatedSelect::new((**select).clone())?))
                }
            };
            Ok(ValidatedCte {
                name: cte.name.clone(),
                columns: cte.columns.clone(),
                recursive: cte.recursive,
                body,
            })
        })
        .collect()
}

fn validate_joins(scope: &QueryScope, query: &SelectQuery) -> Result<Vec<ValidatedJoin>> {
    query
        .joins
        .iter()
        .map(|join| {
            let on = match (&join.on, join.kind.requires_condition()) {
                (Some(on), _) => Some(validate_expr(scope, on, ExprContext::JoinOn)?),
                (None, true) => {
                    return Err(Error::MissingJoinCondition {
                        kind: join.kind.as_sql().to_owned(),
                        dataset: join.dataset.api_name.clone(),
                    });
                }
                (None, false) => None,
            };
            Ok(ValidatedJoin {
                kind: join.kind,
                dataset: join.dataset.clone(),
                on,
            })
        })
        .collect()
}

fn resolve_selection(scope: &QueryScope, fields: &[FieldRef]) -> Result<Vec<ResolvedField>> {
    if fields.is_empty() {
        return scope
            .root()
            .fields
            .iter()
            .filter(|field| field.caps.selectable)
            .map(|field| {
                resolved_from_field(
                    scope,
                    scope.root(),
                    *field,
                    &[],
                    None,
                    default_qualifier(scope, scope.root()),
                    None,
                )
            })
            .collect();
    }

    let mut resolved = Vec::with_capacity(fields.len());
    for field_ref in fields {
        let field = resolve_field_in_scope(scope, field_ref)?;
        if !field.json_path.is_empty() {
            return Err(Error::NotSelectable {
                field: field.display_name(),
            });
        }
        if !field.caps.selectable {
            return Err(Error::NotSelectable {
                field: field.display_name(),
            });
        }
        resolved.push(field);
    }
    Ok(resolved)
}

fn validate_sort(scope: &QueryScope, sort: &Sort) -> Result<ValidatedSort> {
    let field = resolve_field_in_scope(scope, &sort.field)?;
    if !field.json_path.is_empty() || !field.caps.sortable {
        return Err(Error::NotSortable {
            field: field.display_name(),
        });
    }
    Ok(ValidatedSort {
        field,
        dir: sort.dir,
        nulls: sort.nulls,
    })
}

fn resolve_group_by(scope: &QueryScope, fields: &[FieldRef]) -> Result<Vec<ResolvedField>> {
    fields
        .iter()
        .map(|field_ref| {
            let field = resolve_field_in_scope(scope, field_ref)?;
            if field.is_json_path() {
                return Err(Error::NotSelectable {
                    field: field.display_name(),
                });
            }
            Ok(field)
        })
        .collect()
}

fn validate_aggregate(scope: &QueryScope, aggregate: &Aggregate) -> Result<ValidatedAggregate> {
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
            let field = resolve_aggregate_field(scope, "count", field)?;
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
            let field = resolve_aggregate_field(scope, "sum", field)?;
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
            let field = resolve_aggregate_field(scope, "avg", field)?;
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
            let field = resolve_aggregate_field(scope, "min", field)?;
            validate_ordered_aggregate_field("min", &field)?;
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
            let field = resolve_aggregate_field(scope, "max", field)?;
            validate_ordered_aggregate_field("max", &field)?;
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
                .map(|field| resolve_aggregate_field(scope, "json_agg", field))
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
            let field = resolve_aggregate_field(scope, "array_agg", field)?;
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
            let field = resolve_aggregate_field(scope, "string_agg", field)?;
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

fn validate_aggregate_filter(
    scope: &QueryScope,
    filter: &Option<Expr>,
) -> Result<Option<ValidatedExpr>> {
    filter
        .as_ref()
        .map(|filter| validate_expr(scope, filter, ExprContext::Filter))
        .transpose()
}

fn resolve_aggregate_field(
    scope: &QueryScope,
    _aggregate: &str,
    field_ref: &FieldRef,
) -> Result<ResolvedField> {
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

fn validate_ordered_aggregate_field(_aggregate: &str, field: &ResolvedField) -> Result<()> {
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

fn validate_aggregate_aliases(aggregates: &[ValidatedAggregate]) -> Result<()> {
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

fn validate_grouped_selection(
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

fn select_columns(
    selected_fields: &[ResolvedField],
    aggregates: &[ValidatedAggregate],
) -> Vec<SelectColumn> {
    selected_fields
        .iter()
        .cloned()
        .map(SelectColumn::Field)
        .chain(aggregates.iter().map(|aggregate| SelectColumn::Aggregate {
            alias: aggregate.alias().to_owned(),
            ty: aggregate.aggregate_type(),
        }))
        .collect()
}

fn same_resolved_field(left: &ResolvedField, right: &ResolvedField) -> bool {
    left.db_name == right.db_name
        && left.api_name == right.api_name
        && left.qualifier == right.qualifier
        && left.explicit_qualifier == right.explicit_qualifier
        && left.json_path == right.json_path
}

pub(super) fn validate_expr(
    scope: &QueryScope,
    expr: &Expr,
    context: ExprContext,
) -> Result<ValidatedExpr> {
    Ok(match expr {
        Expr::Predicate(predicate) => {
            let field = resolve_field_in_scope(scope, &predicate.field)?;
            if matches!(context, ExprContext::Filter) && !field.caps.filterable {
                return Err(Error::NotFilterable {
                    field: field.display_name(),
                });
            }
            validate_operator(&field, predicate.operator, &predicate.value)?;
            ValidatedExpr::Predicate {
                field,
                operator: predicate.operator,
                value: predicate.value.clone(),
            }
        }
        Expr::ColumnPredicate(predicate) => {
            let left = resolve_field_in_scope(scope, &predicate.left)?;
            let right = resolve_field_in_scope(scope, &predicate.right)?;
            validate_column_operator(&left, predicate.operator, &right)?;
            ValidatedExpr::ColumnPredicate {
                left,
                operator: predicate.operator,
                right,
            }
        }
        Expr::Subquery(predicate) => {
            let field = resolve_field_in_scope(scope, &predicate.field)?;
            if matches!(context, ExprContext::Filter) && !field.caps.filterable {
                return Err(Error::NotFilterable {
                    field: field.display_name(),
                });
            }
            let validated = validate_subquery(scope, &predicate.query)?;
            let selected = validated.columns.len();
            if selected != 1 {
                return Err(Error::InvalidSubquerySelection {
                    expected: 1,
                    actual: selected,
                });
            }
            ValidatedExpr::Subquery {
                field,
                operator: predicate.operator,
                query: Box::new(validated),
            }
        }
        Expr::Exists(predicate) => ValidatedExpr::Exists {
            query: Box::new(validate_subquery(scope, &predicate.query)?),
            negated: predicate.negated,
        },
        Expr::Logical(logical) => {
            if logical.predicates.is_empty() {
                return Err(Error::EmptyLogical {
                    logical: logical.logical.as_str().to_owned(),
                });
            }
            if logical.logical == LogicalOp::Not && logical.predicates.len() != 1 {
                return Err(Error::InvalidNot);
            }
            ValidatedExpr::Logical {
                logical: logical.logical,
                predicates: logical
                    .predicates
                    .iter()
                    .map(|predicate| validate_expr(scope, predicate, context))
                    .collect::<Result<Vec<_>>>()?,
            }
        }
        Expr::Raw(raw) => {
            let placeholders = count_raw_placeholders(&raw.sql);
            if placeholders != raw.binds.len() {
                return Err(Error::RawBindMismatch {
                    placeholders,
                    binds: raw.binds.len(),
                });
            }
            ValidatedExpr::Raw(raw.clone())
        }
    })
}

fn validate_subquery(scope: &QueryScope, query: &SelectQuery) -> Result<ValidatedSelect> {
    let outer_datasets = scope
        .datasets
        .iter()
        .map(|scoped| scoped.dataset.clone())
        .collect::<Vec<_>>();
    ValidatedSelect::new_with_outer_datasets(query.clone(), &outer_datasets)
}
