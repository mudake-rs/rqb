use crate::error::{Error, Result};
use crate::expr::{Expr, LogicalOp};
use crate::request::SelectQuery;

use super::operators::{count_raw_placeholders, validate_operator};
use super::resolve::resolve_field_in_scope;
use super::scope::{ExprContext, QueryScope};
use super::value_type::validate_column_operator;
use super::{ValidatedExpr, ValidatedSelect};

pub(super) fn validate_expr(
    scope: &QueryScope,
    expr: &Expr,
    context: ExprContext,
) -> Result<ValidatedExpr> {
    Ok(match expr {
        Expr::Predicate(predicate) => {
            let field = resolve_field_in_scope(scope, &predicate.field)?;
            if matches!(context, ExprContext::Filter) && !field.caps.filterable {
                return Err(Error::NotFilterable {
                    field: field.display_name(),
                });
            }
            validate_operator(&field, predicate.operator, &predicate.value)?;
            ValidatedExpr::Predicate {
                field,
                operator: predicate.operator,
                value: predicate.value.clone(),
            }
        }
        Expr::ColumnPredicate(predicate) => {
            let left = resolve_field_in_scope(scope, &predicate.left)?;
            let right = resolve_field_in_scope(scope, &predicate.right)?;
            validate_column_operator(&left, predicate.operator, &right)?;
            ValidatedExpr::ColumnPredicate {
                left,
                operator: predicate.operator,
                right,
            }
        }
        Expr::Subquery(predicate) => {
            let field = resolve_field_in_scope(scope, &predicate.field)?;
            if matches!(context, ExprContext::Filter) && !field.caps.filterable {
                return Err(Error::NotFilterable {
                    field: field.display_name(),
                });
            }
            let validated = validate_subquery(scope, &predicate.query)?;
            let selected = validated.columns.len();
            if selected != 1 {
                return Err(Error::InvalidSubquerySelection {
                    expected: 1,
                    actual: selected,
                });
            }
            ValidatedExpr::Subquery {
                field,
                operator: predicate.operator,
                query: Box::new(validated),
            }
        }
        Expr::Exists(predicate) => ValidatedExpr::Exists {
            query: Box::new(validate_subquery(scope, &predicate.query)?),
            negated: predicate.negated,
        },
        Expr::Logical(logical) => {
            if logical.predicates.is_empty() {
                return Err(Error::EmptyLogical {
                    logical: logical.logical.as_str().to_owned(),
                });
            }
            if logical.logical == LogicalOp::Not && logical.predicates.len() != 1 {
                return Err(Error::InvalidNot);
            }
            ValidatedExpr::Logical {
                logical: logical.logical,
                predicates: logical
                    .predicates
                    .iter()
                    .map(|predicate| validate_expr(scope, predicate, context))
                    .collect::<Result<Vec<_>>>()?,
            }
        }
        Expr::Raw(raw) => {
            let placeholders = count_raw_placeholders(&raw.sql);
            if placeholders != raw.binds.len() {
                return Err(Error::RawBindMismatch {
                    placeholders,
                    binds: raw.binds.len(),
                });
            }
            ValidatedExpr::Raw(raw.clone())
        }
    })
}

fn validate_subquery(scope: &QueryScope, query: &SelectQuery) -> Result<ValidatedSelect> {
    let outer_datasets = scope
        .datasets
        .iter()
        .map(|scoped| scoped.dataset.clone())
        .collect::<Vec<_>>();
    ValidatedSelect::new_with_outer_datasets(query.clone(), &outer_datasets)
}
