use std::borrow::Cow;

use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::field::{Field, FieldRef, JsonPathPolicy, ResolvedField};

use super::scope::QueryScope;

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
                dataset: scoped.dataset.api_name.to_string(),
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

    let mut found: Option<(usize, Field)> = None;
    let mut ambiguous: Option<Vec<String>> = None;
    for (idx, scoped) in scope.datasets.iter().enumerate() {
        let Some(candidate) = scoped
            .dataset
            .fields
            .iter()
            .find(|candidate| {
                candidate.api_name == field.api_name || candidate.db_name == field.db_name
            })
            .copied()
        else {
            continue;
        };

        if let Some(matches) = &mut ambiguous {
            matches.push(format!(
                "{}.{}",
                QueryScope::label(&scoped.dataset),
                field.api_name
            ));
        } else if let Some((first_idx, _)) = found {
            let first = &scope.datasets[first_idx].dataset;
            ambiguous = Some(vec![
                format!("{}.{}", QueryScope::label(first), field.api_name),
                format!("{}.{}", QueryScope::label(&scoped.dataset), field.api_name),
            ]);
        } else {
            found = Some((idx, candidate));
        }
    }

    if let Some(matches) = ambiguous {
        return Err(Error::AmbiguousField {
            field: field.api_name.to_owned(),
            matches: matches.join(", "),
        });
    }

    match found {
        Some((idx, found)) => {
            let scoped = &scope.datasets[idx];
            resolved_from_field(
                scope,
                &scoped.dataset,
                found,
                path,
                None,
                default_qualifier(scope, &scoped.dataset),
                alias.clone(),
            )
        }
        None if !scope.has_joins => {
            resolved_from_field(scope, scope.root(), field, path, None, None, alias)
        }
        None => Err(Error::UnknownField {
            dataset: scope.root().api_name.to_string(),
            field: field.api_name.to_owned(),
        }),
    }
}

fn resolve_named_field(
    scope: &QueryScope,
    name: &str,
    alias: Option<String>,
) -> Result<ResolvedField> {
    let mut parts = name
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty());

    let Some(root) = parts.next() else {
        return Err(Error::UnknownField {
            dataset: scope.root().api_name.to_string(),
            field: name.to_owned(),
        });
    };

    let rest = parts.collect::<Vec<_>>();

    if let Some(field_name) = rest.first().copied() {
        match scope.find_qualified(root) {
            Ok(scoped) => {
                let path = rest[1..]
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
                        dataset: scoped.dataset.api_name.to_string(),
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

    let path = rest
        .iter()
        .map(|part| (*part).to_owned())
        .collect::<Vec<_>>();
    let mut found: Option<(usize, Field)> = None;
    let mut ambiguous: Option<Vec<String>> = None;
    for (idx, scoped) in scope.datasets.iter().enumerate() {
        let Some(field) = scoped
            .dataset
            .fields
            .iter()
            .find(|candidate| candidate.api_name == root || candidate.db_name == root)
            .copied()
        else {
            continue;
        };

        if let Some(matches) = &mut ambiguous {
            matches.push(format!("{}.{}", QueryScope::label(&scoped.dataset), root));
        } else if let Some((first_idx, _)) = found {
            let first = &scope.datasets[first_idx].dataset;
            ambiguous = Some(vec![
                format!("{}.{}", QueryScope::label(first), root),
                format!("{}.{}", QueryScope::label(&scoped.dataset), root),
            ]);
        } else {
            found = Some((idx, field));
        }
    }

    if let Some(matches) = ambiguous {
        return Err(Error::AmbiguousField {
            field: name.to_owned(),
            matches: matches.join(", "),
        });
    }

    match found {
        Some((idx, field)) => {
            let scoped = &scope.datasets[idx];
            resolved_from_field(
                scope,
                &scoped.dataset,
                field,
                &path,
                None,
                default_qualifier(scope, &scoped.dataset),
                alias,
            )
        }
        None => Err(Error::UnknownField {
            dataset: scope.root().api_name.to_string(),
            field: name.to_owned(),
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
            api_name: Cow::Borrowed(field.api_name),
            db_name: Cow::Borrowed(field.db_name),
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
        api_name: Cow::Borrowed(field.api_name),
        db_name: Cow::Borrowed(field.db_name),
        ty: field.ty,
        caps,
        json_path: path.to_vec(),
        qualifier,
        explicit_qualifier,
        alias,
    })
}
