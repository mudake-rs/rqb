use crate::error::{Error, Result};
use crate::expr::Sort;

use super::ValidatedSort;
use super::resolve::resolve_field_in_scope;
use super::scope::QueryScope;

pub(super) fn validate_sort(scope: &QueryScope, sort: &Sort) -> Result<ValidatedSort> {
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
