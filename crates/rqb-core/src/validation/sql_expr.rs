use crate::error::{Error, Result};
use crate::field::ResolvedField;
use crate::sql_expr::{SelectItem, SqlExpr};
use crate::types::FieldType;
use crate::value::Value;

use super::expr::validate_expr;
use super::operators::count_raw_placeholders;
use super::resolve::resolve_field_in_scope;
use super::scope::{ExprContext, QueryScope};
use super::{
    ValidatedCaseBranch, ValidatedExpr, ValidatedPredicate, ValidatedSelectItem, ValidatedSqlExpr,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SqlExprContext {
    Select,
    Write,
}

pub(super) fn validate_select_item(
    scope: &QueryScope,
    item: &SelectItem,
) -> Result<ValidatedSelectItem> {
    if item.alias.trim().is_empty() {
        return Err(Error::EmptyExpressionAlias);
    }
    let expr = validate_sql_expr_in_context(scope, &item.expr, SqlExprContext::Select)?;
    let ty = expr.ty();
    Ok(ValidatedSelectItem {
        expr,
        alias: item.alias.clone(),
        ty,
    })
}

pub(super) fn validate_write_sql_expr(
    scope: &QueryScope,
    expr: &SqlExpr,
) -> Result<ValidatedSqlExpr> {
    validate_sql_expr_in_context(scope, expr, SqlExprContext::Write)
}

fn validate_sql_expr_in_context(
    scope: &QueryScope,
    expr: &SqlExpr,
    context: SqlExprContext,
) -> Result<ValidatedSqlExpr> {
    Ok(match expr {
        SqlExpr::Field(field_ref) => {
            let field = resolve_field_in_scope(scope, field_ref)?;
            validate_sql_expr_field(&field, context)?;
            ValidatedSqlExpr::Field(field)
        }
        SqlExpr::Value(value) => {
            let ty = value_type(value).ok_or_else(|| Error::UnknownExpressionType {
                expression: "value".to_owned(),
            })?;
            ValidatedSqlExpr::Value {
                value: value.clone(),
                ty,
            }
        }
        SqlExpr::Raw { raw, ty } => {
            let placeholders = count_raw_placeholders(&raw.sql);
            if placeholders != raw.binds.len() {
                return Err(Error::RawBindMismatch {
                    placeholders,
                    binds: raw.binds.len(),
                });
            }
            ValidatedSqlExpr::Raw {
                raw: raw.clone(),
                ty: *ty,
            }
        }
        SqlExpr::Function { name, args, ty } => ValidatedSqlExpr::Function {
            name: name.clone(),
            args: validate_sql_exprs(scope, args, context)?,
            ty: *ty,
        },
        SqlExpr::Coalesce(args) => {
            if args.is_empty() {
                return Err(Error::UnknownExpressionType {
                    expression: "coalesce".to_owned(),
                });
            }
            let args = validate_sql_exprs(scope, args, context)?;
            let ty = common_expr_type("coalesce", args.iter().map(ValidatedSqlExpr::ty))?;
            ValidatedSqlExpr::Coalesce { args, ty }
        }
        SqlExpr::Case {
            branches,
            otherwise,
        } => {
            if branches.is_empty() {
                return Err(Error::UnknownExpressionType {
                    expression: "case".to_owned(),
                });
            }
            let branches = branches
                .iter()
                .map(|branch| {
                    let condition = validate_expr(scope, &branch.condition, ExprContext::Filter)?;
                    if context == SqlExprContext::Select {
                        validate_selectable_bool_expr_fields(&condition)?;
                    }
                    Ok(ValidatedCaseBranch {
                        condition,
                        value: validate_sql_expr_in_context(scope, &branch.value, context)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let otherwise = Box::new(validate_sql_expr_in_context(scope, otherwise, context)?);
            let ty = common_expr_type(
                "case",
                branches
                    .iter()
                    .map(|branch| branch.value.ty())
                    .chain([otherwise.ty()]),
            )?;
            ValidatedSqlExpr::Case {
                branches,
                otherwise,
                ty,
            }
        }
        SqlExpr::Cast { expr, ty } => ValidatedSqlExpr::Cast {
            expr: Box::new(validate_sql_expr_in_context(scope, expr, context)?),
            ty: *ty,
        },
    })
}

pub(super) fn collect_sql_expr_fields(expr: &ValidatedSqlExpr, output: &mut Vec<ResolvedField>) {
    match expr {
        ValidatedSqlExpr::Field(field) => push_unique_field(output, field.clone()),
        ValidatedSqlExpr::Value { .. } | ValidatedSqlExpr::Raw { .. } => {}
        ValidatedSqlExpr::Function { args, .. } | ValidatedSqlExpr::Coalesce { args, .. } => {
            for arg in args {
                collect_sql_expr_fields(arg, output);
            }
        }
        ValidatedSqlExpr::Case {
            branches,
            otherwise,
            ..
        } => {
            for branch in branches {
                collect_bool_expr_fields(&branch.condition, output);
                collect_sql_expr_fields(&branch.value, output);
            }
            collect_sql_expr_fields(otherwise, output);
        }
        ValidatedSqlExpr::Cast { expr, .. } => collect_sql_expr_fields(expr, output),
    }
}

fn collect_bool_expr_fields(expr: &ValidatedExpr, output: &mut Vec<ResolvedField>) {
    match expr {
        ValidatedExpr::Predicate(predicate) => collect_predicate_fields(predicate, output),
        ValidatedExpr::Logical { predicates, .. } => {
            for predicate in predicates {
                collect_bool_expr_fields(predicate, output);
            }
        }
    }
}

fn collect_predicate_fields(predicate: &ValidatedPredicate, output: &mut Vec<ResolvedField>) {
    match predicate {
        ValidatedPredicate::Raw(_) | ValidatedPredicate::Exists { .. } => {}
        ValidatedPredicate::ColumnBinary { left, right, .. } => {
            push_unique_field(output, left.clone());
            push_unique_field(output, right.clone());
        }
        ValidatedPredicate::Subquery { field, .. }
        | ValidatedPredicate::NullCheck { field, .. }
        | ValidatedPredicate::Binary { field, .. }
        | ValidatedPredicate::NullSafeBinary { field, .. }
        | ValidatedPredicate::In { field, .. }
        | ValidatedPredicate::Between { field, .. }
        | ValidatedPredicate::Like { field, .. }
        | ValidatedPredicate::Regex { field, .. }
        | ValidatedPredicate::TextSearch { field, .. }
        | ValidatedPredicate::ArraySet { field, .. }
        | ValidatedPredicate::ArrayMembership { field, .. }
        | ValidatedPredicate::ArrayState { field, .. }
        | ValidatedPredicate::ArrayElemMatch { field, .. }
        | ValidatedPredicate::JsonKey { field, .. }
        | ValidatedPredicate::JsonKeySet { field, .. }
        | ValidatedPredicate::Containment { field, .. } => push_unique_field(output, field.clone()),
    }
}

fn validate_sql_exprs(
    scope: &QueryScope,
    exprs: &[SqlExpr],
    context: SqlExprContext,
) -> Result<Vec<ValidatedSqlExpr>> {
    exprs
        .iter()
        .map(|expr| validate_sql_expr_in_context(scope, expr, context))
        .collect()
}

fn validate_sql_expr_field(field: &ResolvedField, context: SqlExprContext) -> Result<()> {
    if field.is_json_path() {
        return Err(Error::NotSelectable {
            field: field.display_name(),
        });
    }
    if context == SqlExprContext::Select && !field.caps.selectable {
        return Err(Error::NotSelectable {
            field: field.display_name(),
        });
    }
    Ok(())
}

fn validate_selectable_bool_expr_fields(expr: &ValidatedExpr) -> Result<()> {
    let mut fields = Vec::new();
    collect_bool_expr_fields(expr, &mut fields);
    for field in fields {
        validate_sql_expr_field(&field, SqlExprContext::Select)?;
    }
    Ok(())
}

pub(super) fn common_expr_type<I>(expression: &str, types: I) -> Result<FieldType>
where
    I: IntoIterator<Item = FieldType>,
{
    let mut iter = types.into_iter();
    let Some(first) = iter.next() else {
        return Err(Error::UnknownExpressionType {
            expression: expression.to_owned(),
        });
    };
    let mut common = first;
    for ty in iter {
        common = compatible_type(common, ty).ok_or_else(|| Error::IncompatibleExpressionTypes {
            expression: expression.to_owned(),
            left_type: common.display_name().into_owned(),
            right_type: ty.display_name().into_owned(),
        })?;
    }
    Ok(common)
}

pub(super) fn compatible_type(left: FieldType, right: FieldType) -> Option<FieldType> {
    if left == right {
        return Some(left);
    }
    if left.is_text() && right.is_text() {
        return Some(FieldType::Text);
    }
    if left.is_numeric() && right.is_numeric() {
        return Some(if left == FieldType::Float || right == FieldType::Float {
            FieldType::Float
        } else if left == FieldType::Numeric || right == FieldType::Numeric {
            FieldType::Numeric
        } else {
            FieldType::BigInt
        });
    }
    None
}

fn value_type(value: &Value) -> Option<FieldType> {
    match value {
        Value::Null => None,
        Value::Bool(_) => Some(FieldType::Bool),
        Value::I64(_) => Some(FieldType::BigInt),
        Value::F64(_) => Some(FieldType::Float),
        Value::String(_) => Some(FieldType::Text),
        Value::Bytes(_) => Some(FieldType::Bytea),
        Value::Array(values) => values
            .first()
            .and_then(value_type)
            .and_then(FieldType::array_type_for_scalar),
        Value::Json(_) => Some(FieldType::Jsonb),
    }
}

fn push_unique_field(fields: &mut Vec<ResolvedField>, field: ResolvedField) {
    if fields
        .iter()
        .any(|existing| same_resolved_field(existing, &field))
    {
        return;
    }
    fields.push(field);
}

pub(super) fn same_resolved_field(left: &ResolvedField, right: &ResolvedField) -> bool {
    left.db_name == right.db_name
        && left.api_name == right.api_name
        && left.qualifier == right.qualifier
        && left.explicit_qualifier == right.explicit_qualifier
        && left.json_path == right.json_path
}
