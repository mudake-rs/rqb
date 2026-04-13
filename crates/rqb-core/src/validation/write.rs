use crate::dataset::{Dataset, Source};
use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Expr, Operator};
use crate::field::{FieldRef, ResolvedField};
use crate::value::Value;
use crate::write::{
    ConflictAction, ConflictClause, ConflictTarget, DeleteQuery, InsertQuery, ReturningMode,
    UpdateQuery, WriteAssignment, WriteValue,
};

use super::expr::validate_expr;
use super::operators::count_raw_placeholders;
use super::resolve::resolve_field_in_scope;
use super::scope::{ExprContext, QueryScope};
use super::value_type::{
    enum_type_for_array, enum_type_for_field, require_enum_array, require_enum_scalar,
    validate_column_operator, validate_value_for_field_type,
};
use super::{
    ValidatedAssignment, ValidatedConflictAction, ValidatedConflictClause, ValidatedConflictTarget,
    ValidatedDelete, ValidatedExpr, ValidatedInsert, ValidatedSelect, ValidatedUpdate,
    ValidatedWriteValue,
};

impl ValidatedInsert {
    pub fn new(query: InsertQuery) -> Result<Self> {
        let InsertQuery {
            dataset,
            rows: raw_rows,
            source,
            returning: raw_returning,
            conflict: raw_conflict,
        } = query;

        validate_write_source(&dataset)?;
        let scope = WriteScope::new(&dataset);

        let rows = raw_rows
            .iter()
            .map(|row| validate_assignments(&scope, row))
            .collect::<Result<Vec<_>>>()?;
        let from_select = source
            .as_ref()
            .map(|select| ValidatedSelect::new((**select).clone()))
            .transpose()?;

        match (rows.is_empty(), from_select.is_some()) {
            (true, false) => return Err(Error::EmptyInsert),
            (false, true) => {
                return Err(Error::InvalidValue {
                    field: dataset.api_name.clone(),
                    operator: "insert".to_owned(),
                    message: "cannot combine VALUES and SELECT insert sources".to_owned(),
                });
            }
            _ => {}
        }
        validate_insert_rows_shape(&rows)?;

        let target_fields = match &from_select {
            Some(select) => select
                .selected_fields
                .iter()
                .map(|field| match_write_field_by_name(&scope, &field.api_name, &field.db_name))
                .collect::<Result<Vec<_>>>()?,
            None => rows
                .first()
                .map(|row| {
                    row.iter()
                        .map(|assignment| assignment.field.clone())
                        .collect()
                })
                .unwrap_or_default(),
        };
        if let Some(select) = &from_select {
            for (target, source) in target_fields.iter().zip(&select.selected_fields) {
                validate_column_operator(target, ColumnOperator::Equals, source)?;
            }
        }
        let returning = resolve_returning(&scope, &raw_returning)?;
        let conflict = raw_conflict
            .as_ref()
            .map(|conflict| validate_conflict(&scope, conflict))
            .transpose()?;

        Ok(Self {
            dataset,
            target_fields,
            rows,
            from_select,
            returning,
            conflict,
        })
    }
}

impl ValidatedUpdate {
    pub fn new(query: UpdateQuery) -> Result<Self> {
        let UpdateQuery {
            dataset,
            assignments: raw_assignments,
            filter: raw_filter,
            returning: raw_returning,
        } = query;

        validate_write_source(&dataset)?;
        let scope = WriteScope::new(&dataset);
        if raw_assignments.is_empty() {
            return Err(Error::EmptyUpdate);
        }
        let assignments = validate_assignments(&scope, &raw_assignments)?;
        let filter = raw_filter
            .as_ref()
            .map(|expr| scope.validate_filter(expr))
            .transpose()?;
        let returning = resolve_returning(&scope, &raw_returning)?;
        Ok(Self {
            dataset,
            assignments,
            filter,
            returning,
        })
    }
}

impl ValidatedDelete {
    pub fn new(query: DeleteQuery) -> Result<Self> {
        let DeleteQuery {
            dataset,
            filter: raw_filter,
            returning: raw_returning,
        } = query;

        validate_write_source(&dataset)?;
        let scope = WriteScope::new(&dataset);
        let Some(expr) = &raw_filter else {
            return Err(Error::DeleteWithoutFilter);
        };
        let filter = scope.validate_filter(expr)?;
        let returning = resolve_returning(&scope, &raw_returning)?;
        Ok(Self {
            dataset,
            filter,
            returning,
        })
    }
}

struct WriteScope<'a> {
    dataset: &'a Dataset,
    query_scope: QueryScope,
}

impl<'a> WriteScope<'a> {
    fn new(dataset: &'a Dataset) -> Self {
        Self {
            dataset,
            query_scope: QueryScope::from_dataset(dataset),
        }
    }

    fn validate_filter(&self, expr: &Expr) -> Result<ValidatedExpr> {
        validate_expr(&self.query_scope, expr, ExprContext::Filter)
    }

    fn resolve_field(&self, field_ref: &FieldRef) -> Result<ResolvedField> {
        let field = resolve_field_in_scope(&self.query_scope, field_ref)?;
        if field.is_json_path() {
            return Err(Error::NotSelectable {
                field: field.display_name(),
            });
        }
        Ok(field)
    }
}

fn validate_write_source(dataset: &Dataset) -> Result<()> {
    match dataset.source {
        Source::Table { .. } | Source::View { .. } => Ok(()),
        Source::Cte { .. } | Source::Raw { .. } => Err(Error::UnsupportedWriteSource),
    }
}

fn validate_assignments(
    scope: &WriteScope<'_>,
    assignments: &[WriteAssignment],
) -> Result<Vec<ValidatedAssignment>> {
    assignments
        .iter()
        .map(|assignment| {
            let field = scope.resolve_field(&assignment.field)?;
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
                    let source = scope.resolve_field(source)?;
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

fn resolve_returning(
    scope: &WriteScope<'_>,
    returning: &ReturningMode,
) -> Result<Vec<ResolvedField>> {
    let fields = match returning {
        ReturningMode::None => return Ok(Vec::new()),
        ReturningMode::All => scope
            .dataset
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
            let resolved = scope.resolve_field(field)?;
            if !resolved.caps.selectable {
                return Err(Error::NotSelectable {
                    field: resolved.display_name(),
                });
            }
            Ok(resolved)
        })
        .collect()
}

fn match_write_field_by_name(
    scope: &WriteScope<'_>,
    api_name: &str,
    db_name: &str,
) -> Result<ResolvedField> {
    let field = scope
        .dataset
        .fields
        .iter()
        .find(|field| field.api_name == api_name || field.db_name == db_name)
        .copied()
        .ok_or_else(|| Error::UnknownField {
            dataset: scope.dataset.api_name.clone(),
            field: api_name.to_owned(),
        })?;
    scope.resolve_field(&FieldRef::from(field))
}

fn validate_conflict(
    scope: &WriteScope<'_>,
    conflict: &ConflictClause,
) -> Result<ValidatedConflictClause> {
    let target = match &conflict.target {
        ConflictTarget::Columns(fields) => ValidatedConflictTarget::Columns(
            fields
                .iter()
                .map(|field| scope.resolve_field(field))
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
                .map(|field| scope.resolve_field(field))
                .collect::<Result<Vec<_>>>()?;
            let filter = filter
                .as_ref()
                .map(|expr| scope.validate_filter(expr))
                .transpose()?;
            ValidatedConflictAction::DoUpdate { fields, filter }
        }
    };
    Ok(ValidatedConflictClause { target, action })
}
