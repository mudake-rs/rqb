use super::*;

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
    let alias = field_ref_alias(field.meta, field.qualifier.as_deref());
    SelectItem {
        expr: field.expr(),
        alias,
    }
}

pub(super) fn select_item_for_meta(meta: Meta) -> SelectItem {
    let alias = field_alias(&meta);
    SelectItem {
        expr: ValueExpr::field(meta, None),
        alias,
    }
}

pub(super) fn select_item_for_source_meta(meta: Meta, qualifier: Option<&str>) -> SelectItem {
    let alias = field_alias(&meta);
    SelectItem {
        expr: ValueExpr::field(meta, qualifier.map(str::to_owned)),
        alias,
    }
}

pub(super) fn normalized_assignments(next: impl IntoAssignments) -> Vec<Assignment> {
    let mut assignments = Vec::new();
    extend_assignments(&mut assignments, next.into_assignments());
    assignments
}

pub(super) fn push_returning_fields(source: &Source, items: &mut Vec<SelectItem>) {
    let qualifier = source.explicit_alias().or_else(|| match source {
        Source::Table { name, .. } | Source::View { name, .. } => name.rsplit('.').next(),
        _ => None,
    });
    source.for_each_field(|meta| items.push(select_item_for_source_meta(*meta, qualifier)));
}

pub(super) fn field_alias(meta: &Meta) -> Option<String> {
    (meta.api != meta.db).then(|| meta.api.to_owned())
}

pub(super) fn field_ref_alias(meta: &Meta, qualifier: Option<&str>) -> Option<String> {
    match qualifier {
        Some(qualifier) => Some(format!("{qualifier}_{}", meta.api)),
        None => field_alias(meta),
    }
}
