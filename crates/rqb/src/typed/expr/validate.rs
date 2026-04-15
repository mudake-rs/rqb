use crate::typed::raw;
use crate::{Error, Result};

use super::{BoolExpr, BoolOp, ValueExpr};

impl BoolExpr {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Constant(_) => Ok(()),
            Self::Compare { left, op, right } => {
                validate_compare(left, *op)?;
                left.validate()?;
                right.validate()
            }
            Self::IsNull { expr, .. } => expr.validate(),
            Self::InList { expr, values, .. } => {
                validate_equality_expr(expr, "in")?;
                expr.validate()?;
                for value in values {
                    value.validate()?;
                }
                Ok(())
            }
            Self::InSubquery { expr, query, .. } => {
                validate_equality_expr(expr, "in_subquery")?;
                expr.validate()?;
                query.validate()
            }
            Self::Between {
                expr, low, high, ..
            } => {
                validate_ordered_expr(expr, "between")?;
                expr.validate()?;
                low.validate()?;
                high.validate()
            }
            Self::Like { expr, pattern, .. } => {
                validate_like_expr(expr)?;
                expr.validate()?;
                pattern.validate()
            }
            Self::Regex { expr, pattern, .. } => {
                validate_like_expr(expr)?;
                expr.validate()?;
                pattern.validate()
            }
            Self::Infix {
                left, op, right, ..
            } => {
                validate_infix_expr(left, op)?;
                left.validate()?;
                right.validate()
            }
            Self::Any { value, array, .. } => {
                validate_array_expr(array, "any")?;
                value.validate()?;
                array.validate()
            }
            Self::ArrayIsEmpty { expr, .. } => {
                validate_array_expr(expr, "array_empty")?;
                expr.validate()
            }
            Self::And(exprs) | Self::Or(exprs) => {
                if exprs.is_empty() {
                    return Err(Error::EmptyTypedLogical {
                        logical: match self {
                            Self::And(_) => "and",
                            Self::Or(_) => "or",
                            _ => unreachable!(),
                        }
                        .to_owned(),
                    });
                }
                for expr in exprs {
                    expr.validate()?;
                }
                Ok(())
            }
            Self::Not(expr) => expr.validate(),
            Self::Exists(stmt) => stmt.validate(),
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
        }
    }
}

impl ValueExpr {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Aggregate {
                filter,
                args,
                order_by,
                ..
            } => {
                for arg in args {
                    arg.validate()?;
                }
                for item in order_by {
                    item.validate()?;
                }
                if let Some(filter) = filter {
                    filter.validate()?;
                }
                Ok(())
            }
            Self::Function { args, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                Ok(())
            }
            Self::Case { branches, else_ } => {
                for (when, then) in branches {
                    when.validate()?;
                    then.validate()?;
                }
                if let Some(else_) = else_ {
                    else_.validate()?;
                }
                Ok(())
            }
            Self::Cast { expr, .. } => expr.validate(),
            Self::Binary { left, right, .. } => {
                left.validate()?;
                right.validate()
            }
            Self::Window { args, spec, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                for expr in &spec.partition_by {
                    expr.validate()?;
                }
                for item in &spec.order_by {
                    item.validate()?;
                }
                Ok(())
            }
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
            Self::Subquery(stmt) => stmt.validate(),
            Self::Field { .. } | Self::Excluded(_) | Self::Param(_) => Ok(()),
        }
    }
}

fn validate_compare(left: &ValueExpr, op: BoolOp) -> Result<()> {
    let Some(meta) = left.field_meta() else {
        return Ok(());
    };
    let supported = if op.requires_ordering() {
        meta.ops.ordering
    } else {
        meta.ops.equality
    };
    if supported {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: op.as_name().to_owned(),
    })
}

fn validate_equality_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.equality {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: operator.to_owned(),
    })
}

fn validate_ordered_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.ordering {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: operator.to_owned(),
    })
}

fn validate_like_expr(expr: &ValueExpr) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if matches!(meta.pg, "text" | "varchar" | "bpchar" | "citext") {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: "like".to_owned(),
    })
}

fn validate_infix_expr(expr: &ValueExpr, op: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    let supported = match op {
        "?" | "?|" | "?&" => matches!(meta.pg, "jsonb"),
        "@>" | "<@" | "&&" => {
            meta.pg.ends_with("[]")
                || matches!(
                    meta.pg,
                    "jsonb"
                        | "int4range"
                        | "int8range"
                        | "numrange"
                        | "daterange"
                        | "tsrange"
                        | "tstzrange"
                        | "inet"
                        | "cidr"
                )
        }
        _ => true,
    };
    if supported {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: op.to_owned(),
    })
}

fn validate_array_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.pg.ends_with("[]") {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: operator.to_owned(),
    })
}
