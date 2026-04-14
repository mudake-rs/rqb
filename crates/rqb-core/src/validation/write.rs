use crate::aggregate::{AggregateType, SelectColumn};
use crate::dataset::{Dataset, Source};
use crate::error::{Error, Result};
use crate::expr::{ColumnOperator, Expr, Operator};
use crate::field::{FieldRef, ResolvedField};
use crate::types::{FieldType, TypeFamily, TypeSpec};
use crate::value::Value;
use crate::write::{
    ConflictAction, ConflictClause, ConflictTarget, DeleteQuery, InsertQuery, ReturningFields,
    ReturningMode, UpdateQuery, WriteAssignment, WriteValue,
};

use super::expr::validate_expr;
use super::operators::count_raw_placeholders;
use super::resolve::resolve_field_in_scope;
use super::scope::{ExprContext, QueryScope};
use super::sql_expr::{collect_sql_expr_fields, validate_select_item, validate_write_sql_expr};
use super::value_type::{
    enum_type_for_array, enum_type_for_field, require_enum_array, require_enum_scalar,
    validate_column_operator, validate_value_for_field_type,
};
use super::{
    ValidatedAssignment, ValidatedConflictAction, ValidatedConflictClause, ValidatedConflictTarget,
    ValidatedDelete, ValidatedExpr, ValidatedInsert, ValidatedReturningItem, ValidatedSelect,
    ValidatedUpdate, ValidatedWriteValue,
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
            .map(|row| validate_assignments(&scope, row, WriteExprContext::Insert))
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
                .columns
                .iter()
                .map(|column| {
                    let alias = column.alias();
                    match_write_field_by_name(&scope, &alias, &alias)
                })
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
            for (target, source) in target_fields.iter().zip(&select.columns) {
                validate_insert_select_column_type(target, source)?;
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
        let assignments = validate_assignments(&scope, &raw_assignments, WriteExprContext::Update)?;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteExprContext {
    Insert,
    Update,
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
    context: WriteExprContext,
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
                WriteValue::Expr(expr) => {
                    let expr = validate_write_sql_expr(&scope.query_scope, expr)?;
                    validate_write_expr_context(&field, &expr, context)?;
                    validate_write_expr_type(&field, &expr)?;
                    ValidatedWriteValue::Expr(expr)
                }
                WriteValue::Default => ValidatedWriteValue::Default,
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

fn validate_write_expr_context(
    field: &ResolvedField,
    expr: &super::ValidatedSqlExpr,
    context: WriteExprContext,
) -> Result<()> {
    if context == WriteExprContext::Update {
        return Ok(());
    }
    let mut fields = Vec::new();
    collect_sql_expr_fields(expr, &mut fields);
    if fields.is_empty() {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: "insert".to_owned(),
        message: "insert expressions cannot reference target fields".to_owned(),
    })
}

fn validate_write_expr_type(field: &ResolvedField, expr: &super::ValidatedSqlExpr) -> Result<()> {
    let expr_type = expr.ty();
    if write_expr_type_assignable(field.ty, expr_type) {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: "write".to_owned(),
        message: format!(
            "expected expression compatible with {}, got {}",
            field.ty.display_name(),
            expr_type.display_name()
        ),
    })
}

fn write_expr_type_assignable(target: FieldType, expr: FieldType) -> bool {
    if target == expr {
        return true;
    }
    if target.is_text() && expr.is_text() {
        return true;
    }
    match target {
        FieldType::Integer | FieldType::BigInt => {
            matches!(expr, FieldType::Integer | FieldType::BigInt)
        }
        FieldType::Numeric => matches!(
            expr,
            FieldType::Integer | FieldType::BigInt | FieldType::Numeric
        ),
        FieldType::Float => expr.is_numeric(),
        FieldType::Custom(type_spec) => target_custom_type_assignable(*type_spec, expr),
        _ => false,
    }
}

fn target_custom_type_assignable(type_spec: TypeSpec, expr: FieldType) -> bool {
    if matches!(expr, FieldType::Custom(expr_spec) if *expr_spec == type_spec) {
        return true;
    }
    match type_spec.family {
        TypeFamily::Text => expr.is_text(),
        TypeFamily::Numeric => matches!(
            expr,
            FieldType::Integer | FieldType::BigInt | FieldType::Numeric
        ),
        TypeFamily::Bool => expr == FieldType::Bool,
        TypeFamily::Jsonb => expr == FieldType::Jsonb,
        TypeFamily::Bytes => expr == FieldType::Bytea,
        TypeFamily::Uuid
        | TypeFamily::Timestamp
        | TypeFamily::Timestamptz
        | TypeFamily::Date
        | TypeFamily::Network
        | TypeFamily::Range => false,
    }
}

fn validate_insert_select_column_type(field: &ResolvedField, column: &SelectColumn) -> Result<()> {
    if let SelectColumn::Field(source) = column {
        return validate_column_operator(field, ColumnOperator::Equals, source);
    }
    let source_type = select_column_type(column);
    if write_expr_type_assignable(field.ty, source_type) {
        return Ok(());
    }
    Err(Error::InvalidValue {
        field: field.display_name(),
        operator: "insert".to_owned(),
        message: format!(
            "expected source column compatible with {}, got {}",
            field.ty.display_name(),
            source_type.display_name()
        ),
    })
}

fn select_column_type(column: &SelectColumn) -> FieldType {
    match column {
        SelectColumn::Field(field) => field.ty,
        SelectColumn::Aggregate { ty, .. } => aggregate_type_to_field_type(ty),
        SelectColumn::Expression { ty, .. } => *ty,
    }
}

fn aggregate_type_to_field_type(ty: &AggregateType) -> FieldType {
    match ty {
        AggregateType::Count => FieldType::BigInt,
        AggregateType::Sum | AggregateType::Avg => FieldType::Float,
        AggregateType::Min(ty) | AggregateType::Max(ty) => *ty,
        AggregateType::Json => FieldType::Jsonb,
        AggregateType::String => FieldType::Text,
    }
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
) -> Result<Vec<ValidatedReturningItem>> {
    let fields = match returning {
        ReturningMode {
            fields: ReturningFields::None,
            expressions,
        } if expressions.is_empty() => return Ok(Vec::new()),
        ReturningMode {
            fields: ReturningFields::None,
            ..
        } => Vec::new(),
        ReturningMode {
            fields: ReturningFields::All,
            ..
        } => scope
            .dataset
            .fields
            .iter()
            .copied()
            .filter(|field| field.caps.selectable)
            .map(FieldRef::from)
            .collect::<Vec<_>>(),
        ReturningMode {
            fields: ReturningFields::Fields(fields),
            ..
        } => fields.clone(),
    };

    let mut items = fields
        .iter()
        .map(|field| {
            let resolved = scope.resolve_field(field)?;
            if !resolved.caps.selectable {
                return Err(Error::NotSelectable {
                    field: resolved.display_name(),
                });
            }
            Ok(ValidatedReturningItem::Field(resolved))
        })
        .collect::<Result<Vec<_>>>()?;

    for item in &returning.expressions {
        items.push(ValidatedReturningItem::Expression(validate_select_item(
            &scope.query_scope,
            item,
        )?));
    }
    validate_returning_aliases(&items)?;
    Ok(items)
}

fn validate_returning_aliases(items: &[ValidatedReturningItem]) -> Result<()> {
    let mut aliases = std::collections::HashSet::with_capacity(items.len());
    for item in items {
        let alias = item.alias();
        if !aliases.insert(alias.clone()) {
            return Err(Error::DuplicateOutputAlias { alias });
        }
    }
    Ok(())
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
