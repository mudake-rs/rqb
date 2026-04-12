use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::field::{Field, FieldRef, JsonPathPolicy, ResolvedField};
use crate::request::SelectQuery;

use super::scope::QueryScope;

pub fn resolve_field(dataset: &Dataset, field_ref: &FieldRef) -> Result<ResolvedField> {
    let query = SelectQuery::new(dataset.clone());
    let scope = QueryScope::new(&query)?;
    resolve_field_in_scope(&scope, field_ref)
}

pub fn resolve_query_field(query: &SelectQuery, field_ref: &FieldRef) -> Result<ResolvedField> {
    let scope = QueryScope::new(query)?;
    resolve_field_in_scope(&scope, field_ref)
}

pub fn resolve_query_field_with_outer(
    query: &SelectQuery,
    outer_datasets: &[Dataset],
    field_ref: &FieldRef,
) -> Result<ResolvedField> {
    let scope = QueryScope::new_with_outer(query, outer_datasets)?;
    resolve_field_in_scope(&scope, field_ref)
}

pub(super) fn resolve_field_in_scope(
    scope: &QueryScope,
    field_ref: &FieldRef,
) -> Result<ResolvedField> {
    match field_ref {
        FieldRef::Known {
            qualifier,
            field,
            path,
            alias,
        } => resolve_known_field(scope, qualifier.as_deref(), *field, path, alias.clone()),
        FieldRef::Named { name, alias } => resolve_named_field(scope, name, alias.clone()),
    }
}

fn resolve_known_field(
    scope: &QueryScope,
    qualifier: Option<&str>,
    field: Field,
    path: &[String],
    alias: Option<String>,
) -> Result<ResolvedField> {
    if let Some(qualifier) = qualifier {
        let scoped = scope.find_qualified(qualifier)?;
        let found = scoped
            .dataset
            .fields
            .iter()
            .find(|candidate| {
                candidate.api_name == field.api_name || candidate.db_name == field.db_name
            })
            .copied()
            .ok_or_else(|| Error::UnknownField {
                dataset: scoped.dataset.api_name.clone(),
                field: field.api_name.to_owned(),
            })?;
        return resolved_from_field(
            scope,
            &scoped.dataset,
            found,
            path,
            Some(qualifier.to_owned()),
            Some(scoped.dataset.sql_qualifier().to_owned()),
            alias,
        );
    }

    let matches = scope
        .datasets
        .iter()
        .filter_map(|scoped| {
            scoped
                .dataset
                .fields
                .iter()
                .find(|candidate| {
                    candidate.api_name == field.api_name || candidate.db_name == field.db_name
                })
                .copied()
                .map(|found| (scoped, found))
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [(scoped, found)] => resolved_from_field(
            scope,
            &scoped.dataset,
            *found,
            path,
            None,
            default_qualifier(scope, &scoped.dataset),
            alias.clone(),
        ),
        [] if !scope.has_joins => {
            resolved_from_field(scope, scope.root(), field, path, None, None, alias)
        }
        [] => Err(Error::UnknownField {
            dataset: scope.root().api_name.clone(),
            field: field.api_name.to_owned(),
        }),
        many => Err(Error::AmbiguousField {
            field: field.api_name.to_owned(),
            matches: many
                .iter()
                .map(|(scoped, _)| {
                    format!("{}.{}", QueryScope::label(&scoped.dataset), field.api_name)
                })
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

fn resolve_named_field(
    scope: &QueryScope,
    name: &str,
    alias: Option<String>,
) -> Result<ResolvedField> {
    let parts = name
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    let Some(root) = parts.first().copied() else {
        return Err(Error::UnknownField {
            dataset: scope.root().api_name.clone(),
            field: name.to_owned(),
        });
    };

    if parts.len() >= 2 {
        match scope.find_qualified(root) {
            Ok(scoped) => {
                let field_name = parts[1];
                let path = parts[2..]
                    .iter()
                    .map(|part| (*part).to_owned())
                    .collect::<Vec<_>>();
                let field = scoped
                    .dataset
                    .fields
                    .iter()
                    .find(|candidate| {
                        candidate.api_name == field_name || candidate.db_name == field_name
                    })
                    .copied()
                    .ok_or_else(|| Error::UnknownField {
                        dataset: scoped.dataset.api_name.clone(),
                        field: name.to_owned(),
                    })?;
                return resolved_from_field(
                    scope,
                    &scoped.dataset,
                    field,
                    &path,
                    Some(root.to_owned()),
                    Some(scoped.dataset.sql_qualifier().to_owned()),
                    alias,
                );
            }
            Err(Error::UnknownDatasetQualifier { .. }) => {}
            Err(error) => return Err(error),
        }
    }

    let path = parts[1..]
        .iter()
        .map(|part| (*part).to_owned())
        .collect::<Vec<_>>();
    let matches = scope
        .datasets
        .iter()
        .filter_map(|scoped| {
            scoped
                .dataset
                .fields
                .iter()
                .find(|candidate| candidate.api_name == root || candidate.db_name == root)
                .copied()
                .map(|field| (scoped, field))
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [(scoped, field)] => resolved_from_field(
            scope,
            &scoped.dataset,
            *field,
            &path,
            None,
            default_qualifier(scope, &scoped.dataset),
            alias,
        ),
        [] => Err(Error::UnknownField {
            dataset: scope.root().api_name.clone(),
            field: name.to_owned(),
        }),
        many => Err(Error::AmbiguousField {
            field: name.to_owned(),
            matches: many
                .iter()
                .map(|(scoped, _)| format!("{}.{}", QueryScope::label(&scoped.dataset), root))
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

pub(super) fn default_qualifier(scope: &QueryScope, dataset: &Dataset) -> Option<String> {
    scope.has_joins.then(|| dataset.sql_qualifier().to_owned())
}

pub(super) fn resolved_from_field(
    _scope: &QueryScope,
    _dataset: &Dataset,
    field: Field,
    path: &[String],
    explicit_qualifier: Option<String>,
    qualifier: Option<String>,
    alias: Option<String>,
) -> Result<ResolvedField> {
    if path.is_empty() {
        return Ok(ResolvedField {
            api_name: field.api_name.to_owned(),
            db_name: field.db_name.to_owned(),
            ty: field.ty,
            caps: field.caps,
            json_path: Vec::new(),
            qualifier,
            explicit_qualifier,
            alias,
        });
    }

    let display = format!("{}.{}", field.api_name, path.join("."));
    if !field.ty.is_jsonb() {
        return Err(Error::NotJsonbPath {
            field: field.api_name.to_owned(),
            path: display,
        });
    }
    if field.caps.json_path == JsonPathPolicy::Deny {
        return Err(Error::JsonbPathDenied {
            field: field.api_name.to_owned(),
        });
    }

    let mut caps = field.caps;
    caps.selectable = false;
    caps.sortable = false;

    Ok(ResolvedField {
        api_name: field.api_name.to_owned(),
        db_name: field.db_name.to_owned(),
        ty: field.ty,
        caps,
        json_path: path.to_vec(),
        qualifier,
        explicit_qualifier,
        alias,
    })
}
