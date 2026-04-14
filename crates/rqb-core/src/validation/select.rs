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
use super::sql_expr::{collect_sql_expr_fields, validate_select_item};
use super::{
    ValidatedAggregate, ValidatedCte, ValidatedCteBody, ValidatedJoin, ValidatedSelect,
    ValidatedSelectItem,
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

        let mut selected_fields = resolve_selection(&scope, &query)?;
        apply_root_output_aliases(&query, &mut selected_fields);
        let distinct_on = resolve_distinct_on(&scope, &query.distinct_on)?;
        let aggregates = query
            .aggregates
            .iter()
            .map(|aggregate| validate_aggregate(&scope, aggregate))
            .collect::<Result<Vec<_>>>()?;
        let select_items = query
            .select_items
            .iter()
            .map(|item| validate_select_item(&scope, item))
            .collect::<Result<Vec<_>>>()?;
        let mut expression_fields = Vec::new();
        for item in &select_items {
            collect_sql_expr_fields(&item.expr, &mut expression_fields);
        }
        let group_by = if !query.group_by.is_empty() {
            resolve_group_by(&scope, &query.group_by)?
        } else if !aggregates.is_empty() {
            let mut group_by = selected_fields.clone();
            for field in &expression_fields {
                if !group_by
                    .iter()
                    .any(|existing| same_resolved_field(existing, field))
                {
                    group_by.push(field.clone());
                }
            }
            group_by
        } else {
            Vec::new()
        };
        validate_aggregate_aliases(&aggregates)?;
        validate_grouped_selection(&selected_fields, &group_by, &aggregates)?;
        validate_grouped_expression_selection(&expression_fields, &group_by, &aggregates)?;
        let columns = select_columns(&selected_fields, &aggregates, &select_items);
        validate_output_aliases(&columns)?;
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
            select_items,
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
                CteBody::Query(query) => ValidatedCteBody::Query(Box::new(
                    super::ValidatedQueryExpr::new((**query).clone())?,
                )),
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

fn resolve_selection(scope: &QueryScope, query: &SelectQuery) -> Result<Vec<ResolvedField>> {
    let fields = &query.request.fields;
    if fields.is_empty() && query.aggregates.is_empty() && query.select_items.is_empty() {
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
    select_items: &[ValidatedSelectItem],
) -> Vec<SelectColumn> {
    selected_fields
        .iter()
        .cloned()
        .map(SelectColumn::Field)
        .chain(aggregates.iter().map(|aggregate| SelectColumn::Aggregate {
            alias: aggregate.alias().to_owned(),
            ty: aggregate.aggregate_type(),
        }))
        .chain(select_items.iter().map(|item| SelectColumn::Expression {
            alias: item.alias.clone(),
            ty: item.ty,
        }))
        .collect()
}

fn validate_output_aliases(columns: &[SelectColumn]) -> Result<()> {
    let mut aliases = std::collections::HashSet::with_capacity(columns.len());
    for column in columns {
        let alias = column.alias();
        if !aliases.insert(alias.clone()) {
            return Err(Error::DuplicateOutputAlias { alias });
        }
    }
    Ok(())
}

fn validate_grouped_expression_selection(
    expression_fields: &[ResolvedField],
    group_by: &[ResolvedField],
    aggregates: &[ValidatedAggregate],
) -> Result<()> {
    if group_by.is_empty() || aggregates.is_empty() {
        return Ok(());
    }
    for field in expression_fields {
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

fn same_resolved_field(left: &ResolvedField, right: &ResolvedField) -> bool {
    left.db_name == right.db_name
        && left.api_name == right.api_name
        && left.qualifier == right.qualifier
        && left.explicit_qualifier == right.explicit_qualifier
        && left.json_path == right.json_path
}
