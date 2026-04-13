use crate::dataset::{Dataset, Source};
use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Expr, Operator};
use crate::field::{FieldRef, ResolvedField};
use crate::request::SelectQuery;
use crate::value::Value;
use crate::write::{
    ConflictAction, ConflictClause, ConflictTarget, DeleteQuery, InsertQuery, ReturningMode,
    UpdateQuery, WriteAssignment, WriteValue,
};

use super::operators::{
    count_raw_placeholders, enum_type_for_array, enum_type_for_field, require_enum_array,
    require_enum_scalar, validate_column_operator, validate_value_for_field_type,
};
use super::resolve::resolve_field_in_scope;
use super::scope::{ExprContext, QueryScope};
use super::select::validate_expr;
use super::{
    ValidatedAssignment, ValidatedConflictAction, ValidatedConflictClause, ValidatedConflictTarget,
    ValidatedDelete, ValidatedExpr, ValidatedInsert, ValidatedSelect, ValidatedUpdate,
    ValidatedWriteValue,
};

impl ValidatedInsert {
    pub fn new(query: InsertQuery) -> Result<Self> {
        validate_write_source(&query.dataset)?;

        let rows = query
            .rows
            .iter()
            .map(|row| validate_assignments(&query.dataset, row))
            .collect::<Result<Vec<_>>>()?;
        let from_select = query
            .source
            .as_ref()
            .map(|select| ValidatedSelect::new((**select).clone()))
            .transpose()?;

        match (rows.is_empty(), from_select.is_some()) {
            (true, false) => return Err(Error::EmptyInsert),
            (false, true) => {
                return Err(Error::InvalidValue {
                    field: query.dataset.api_name.clone(),
                    operator: "insert".to_owned(),
                    message: "cannot combine VALUES and SELECT insert sources".to_owned(),
                });
            }
            _ => {}
        }
        validate_insert_rows_shape(&rows)?;

        let from_select_targets = match &from_select {
            Some(select) => select
                .selected_fields
                .iter()
                .map(|field| {
                    match_write_field_by_name(&query.dataset, &field.api_name, &field.db_name)
                })
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        if let Some(select) = &from_select {
            for (target, source) in from_select_targets.iter().zip(&select.selected_fields) {
                validate_column_operator(target, ColumnOperator::Equals, source)?;
            }
        }
        let returning = resolve_returning(&query.dataset, &query.returning)?;
        let conflict = query
            .conflict
            .as_ref()
            .map(|conflict| validate_conflict(&query.dataset, conflict))
            .transpose()?;

        Ok(Self {
            query,
            rows,
            from_select,
            from_select_targets,
            returning,
            conflict,
        })
    }
}

impl ValidatedUpdate {
    pub fn new(query: UpdateQuery) -> Result<Self> {
        validate_write_source(&query.dataset)?;
        if query.assignments.is_empty() {
            return Err(Error::EmptyUpdate);
        }
        let assignments = validate_assignments(&query.dataset, &query.assignments)?;
        let filter = query
            .filter
            .as_ref()
            .map(|expr| validate_write_filter(&query.dataset, expr))
            .transpose()?;
        let returning = resolve_returning(&query.dataset, &query.returning)?;
        Ok(Self {
            query,
            assignments,
            filter,
            returning,
        })
    }
}

impl ValidatedDelete {
    pub fn new(query: DeleteQuery) -> Result<Self> {
        validate_write_source(&query.dataset)?;
        let Some(expr) = &query.filter else {
            return Err(Error::DeleteWithoutFilter);
        };
        let filter = validate_write_filter(&query.dataset, expr)?;
        let returning = resolve_returning(&query.dataset, &query.returning)?;
        Ok(Self {
            query,
            filter,
            returning,
        })
    }
}

fn validate_write_filter(dataset: &Dataset, expr: &Expr) -> Result<ValidatedExpr> {
    let select = SelectQuery::new(dataset.clone());
    let scope = QueryScope::new(&select)?;
    validate_expr(&scope, expr, ExprContext::Filter)
}

fn validate_write_source(dataset: &Dataset) -> Result<()> {
    match dataset.source {
        Source::Table { .. } | Source::View { .. } => Ok(()),
        Source::Cte { .. } | Source::Raw { .. } => Err(Error::UnsupportedWriteSource),
    }
}

fn validate_assignments(
    dataset: &Dataset,
    assignments: &[WriteAssignment],
) -> Result<Vec<ValidatedAssignment>> {
    assignments
        .iter()
        .map(|assignment| {
            let field = resolve_write_field(dataset, &assignment.field)?;
            let value = match &assignment.value {
                WriteValue::Value(value) => {
                    validate_write_value(&field, value)?;
                    ValidatedWriteValue::Value(value.clone())
                }
                WriteValue::Raw(raw) => {
                    let placeholders = count_raw_placeholders(&raw.sql);
                    if placeholders != raw.binds.len() {
                        return Err(Error::RawBindMismatch {
                            placeholders,
                            binds: raw.binds.len(),
                        });
                    }
                    ValidatedWriteValue::Raw(raw.clone())
                }
                WriteValue::Column(source) => {
                    let source = resolve_write_field(dataset, source)?;
                    validate_column_operator(&field, ColumnOperator::Equals, &source)?;
                    ValidatedWriteValue::Column(source)
                }
            };
            Ok(ValidatedAssignment { field, value })
        })
        .collect()
}

fn validate_write_value(field: &ResolvedField, value: &Value) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    if let Some(enum_type) = enum_type_for_field(field) {
        return require_enum_scalar(field, Operator::Equals, enum_type, value);
    }
    if let Some(enum_type) = enum_type_for_array(field) {
        return require_enum_array(field, Operator::ArrayContainsAll, enum_type, value);
    }
    validate_value_for_field_type(field, "write", value)?;
    Ok(())
}

fn validate_insert_rows_shape(rows: &[Vec<ValidatedAssignment>]) -> Result<()> {
    let Some(first) = rows.first() else {
        return Ok(());
    };
    let first_fields = first
        .iter()
        .map(|assignment| assignment.field.db_name.as_str())
        .collect::<Vec<_>>();
    for row in rows.iter().skip(1) {
        let fields = row
            .iter()
            .map(|assignment| assignment.field.db_name.as_str())
            .collect::<Vec<_>>();
        if fields != first_fields {
            return Err(Error::InconsistentInsertFields);
        }
    }
    Ok(())
}

fn resolve_returning(dataset: &Dataset, returning: &ReturningMode) -> Result<Vec<ResolvedField>> {
    let fields = match returning {
        ReturningMode::None => return Ok(Vec::new()),
        ReturningMode::All => dataset
            .fields
            .iter()
            .copied()
            .filter(|field| field.caps.selectable)
            .map(FieldRef::from)
            .collect::<Vec<_>>(),
        ReturningMode::Fields(fields) => fields.clone(),
    };

    fields
        .iter()
        .map(|field| {
            let resolved = resolve_write_field(dataset, field)?;
            if !resolved.caps.selectable {
                return Err(Error::NotSelectable {
                    field: resolved.display_name(),
                });
            }
            Ok(resolved)
        })
        .collect()
}

fn resolve_write_field(dataset: &Dataset, field_ref: &FieldRef) -> Result<ResolvedField> {
    let query = SelectQuery::new(dataset.clone());
    let scope = QueryScope::new(&query)?;
    let field = resolve_field_in_scope(&scope, field_ref)?;
    if field.is_json_path() {
        return Err(Error::NotSelectable {
            field: field.display_name(),
        });
    }
    Ok(field)
}

fn match_write_field_by_name(
    dataset: &Dataset,
    api_name: &str,
    db_name: &str,
) -> Result<ResolvedField> {
    let field = dataset
        .fields
        .iter()
        .find(|field| field.api_name == api_name || field.db_name == db_name)
        .copied()
        .ok_or_else(|| Error::UnknownField {
            dataset: dataset.api_name.clone(),
            field: api_name.to_owned(),
        })?;
    resolve_write_field(dataset, &FieldRef::from(field))
}

fn validate_conflict(
    dataset: &Dataset,
    conflict: &ConflictClause,
) -> Result<ValidatedConflictClause> {
    let target = match &conflict.target {
        ConflictTarget::Columns(fields) => ValidatedConflictTarget::Columns(
            fields
                .iter()
                .map(|field| resolve_write_field(dataset, field))
                .collect::<Result<Vec<_>>>()?,
        ),
        ConflictTarget::Constraint(constraint) => {
            ValidatedConflictTarget::Constraint(constraint.clone())
        }
    };
    let action = match &conflict.action {
        ConflictAction::DoNothing => ValidatedConflictAction::DoNothing,
        ConflictAction::DoUpdate { fields, filter } => {
            let fields = fields
                .iter()
                .map(|field| resolve_write_field(dataset, field))
                .collect::<Result<Vec<_>>>()?;
            let filter = filter
                .as_ref()
                .map(|expr| validate_write_filter(dataset, expr))
                .transpose()?;
            ValidatedConflictAction::DoUpdate { fields, filter }
        }
    };
    Ok(ValidatedConflictClause { target, action })
}
