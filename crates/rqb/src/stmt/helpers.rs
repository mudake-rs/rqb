use super::*;

pub(super) fn extend_insert_assignments(
    columns: &mut Vec<Meta>,
    assignments: &mut Vec<Assignment>,
    next: Vec<Assignment>,
) {
    for assignment in next {
        push_column(columns, assignment.field);
        push_assignment(assignments, assignment);
    }
}

pub(super) fn extend_assignments(assignments: &mut Vec<Assignment>, next: Vec<Assignment>) {
    for assignment in next {
        push_assignment(assignments, assignment);
    }
}

pub(super) fn push_column(columns: &mut Vec<Meta>, field: Meta) {
    if !columns.iter().any(|existing| existing.db == field.db) {
        columns.push(field);
    }
}

pub(super) fn push_assignment(assignments: &mut Vec<Assignment>, assignment: Assignment) {
    assignments.retain(|existing| existing.field.db != assignment.field.db);
    assignments.push(assignment);
}

pub(super) fn select_item_for_field<T>(field: Field<T>) -> SelectItem {
    let alias = field_alias(field.meta);
    SelectItem {
        expr: field.expr(),
        alias,
    }
}

pub(super) fn select_item_for_ref<T>(field: FieldRef<T>) -> SelectItem {
    let alias = field_ref_alias(&field);
    SelectItem {
        expr: field.expr(),
        alias,
    }
}

pub(super) fn select_item_for_meta(meta: Meta) -> SelectItem {
    let alias = field_alias(&meta);
    SelectItem {
        expr: ValueExpr::Field {
            meta,
            qualifier: None,
        },
        alias,
    }
}

pub(super) fn push_all_source_fields(source: &Source, items: &mut Vec<SelectItem>) {
    source.for_each_field(|meta| items.push(select_item_for_meta(*meta)));
}

pub(super) fn field_alias(meta: &Meta) -> Option<String> {
    (meta.api != meta.db).then(|| meta.api.to_owned())
}

pub(super) fn field_ref_alias<T>(field: &FieldRef<T>) -> Option<String> {
    match &field.qualifier {
        Some(qualifier) => Some(format!("{qualifier}_{}", field.meta.api)),
        None => field_alias(field.meta),
    }
}
