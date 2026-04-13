use crate::aggregate::SelectColumn;
use crate::dataset::{CteBody, Dataset};
use crate::error::{Error, Result};
use crate::field::{FieldRef, ResolvedField};
use crate::request::SelectQuery;

use super::aggregate::{
    validate_aggregate, validate_aggregate_aliases, validate_grouped_selection,
};
use super::expr::validate_expr;
use super::operators::count_raw_placeholders;
use super::resolve::{default_qualifier, resolve_field_in_scope, resolved_from_field};
use super::scope::{ExprContext, QueryScope};
use super::sort::validate_sort;
use super::{ValidatedAggregate, ValidatedCte, ValidatedCteBody, ValidatedJoin, ValidatedSelect};

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
