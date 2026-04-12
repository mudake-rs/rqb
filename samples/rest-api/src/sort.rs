use rqb::prelude::{FieldRef, Sort};

use crate::error::AppError;

pub fn parse_sort(input: Option<&str>, default: Sort) -> Result<Sort, AppError> {
    let Some(input) = input.map(str::trim).filter(|input| !input.is_empty()) else {
        return Ok(default);
    };

    // This parser only checks transport syntax. Whether the field exists and is sortable is left
    // to rqb validation, so CLI-generated schema remains the source of truth.
    let (field_name, dir) = input.split_once(':').ok_or_else(|| {
        AppError::BadRequest(format!(
            "invalid sort format `{input}`, expected `field:asc` or `field:desc`"
        ))
    })?;
    let field_name = field_name.trim();
    let dir = dir.trim();
    if field_name.is_empty() {
        return Err(AppError::BadRequest(format!(
            "invalid sort format `{input}`, field name is empty"
        )));
    }

    let field = FieldRef::named(field_name);
    if dir.eq_ignore_ascii_case("asc") {
        Ok(Sort::asc(field))
    } else if dir.eq_ignore_ascii_case("desc") {
        Ok(Sort::desc(field))
    } else {
        Err(AppError::BadRequest(format!(
            "invalid sort direction `{dir}`, expected `asc` or `desc`"
        )))
    }
}
