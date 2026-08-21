use crate::raw;
use crate::{Error, Result};

use super::{BoolExpr, BoolOp, ValueExpr};

impl BoolExpr {
    /// Validates this boolean expression before SQL rendering.
    pub fn validate(&self) -> Result<()> {
        match self {
            BoolExpr::Constant(_) => Ok(()),
            BoolExpr::Compare { left, op, right } => {
                validate_compare(left, *op)?;
                validate_row_compare(left, right)?;
                left.validate()?;
                right.validate()
            }
            BoolExpr::IsNull { expr, .. } | BoolExpr::IsBoolean { expr, .. } => expr.validate(),
            BoolExpr::InList { expr, values, .. } => {
                validate_equality_expr(expr, "in")?;
                expr.validate()?;
                for value in values {
                    value.validate()?;
                }
                Ok(())
            }
            BoolExpr::InSubquery { expr, query, .. } => {
                validate_equality_expr(expr, "in_subquery")?;
                expr.validate()?;
                query.validate_query_statement("IN subquery must be SELECT, set, or raw statement")
            }
            BoolExpr::Between {
                expr, low, high, ..
            } => {
                validate_ordered_expr(expr, "between")?;
                expr.validate()?;
                low.validate()?;
                high.validate()
            }
            BoolExpr::Like { expr, pattern, .. } => {
                validate_pattern_expr(expr, "like")?;
                expr.validate()?;
                pattern.validate()
            }
            BoolExpr::SimilarTo { expr, pattern, .. } => {
                validate_pattern_expr(expr, "similar_to")?;
                expr.validate()?;
                pattern.validate()
            }
            BoolExpr::Regex { expr, pattern, .. } => {
                validate_pattern_expr(expr, "regex")?;
                expr.validate()?;
                pattern.validate()
            }
            BoolExpr::Infix {
                left, op, right, ..
            } => {
                validate_infix_expr(left, op)?;
                left.validate()?;
                right.validate()
            }
            BoolExpr::Any { value, array, .. } => {
                validate_array_expr(array, "any")?;
                value.validate()?;
                array.validate()
            }
            BoolExpr::ArrayIsEmpty { expr, .. } => {
                validate_array_expr(expr, "array_empty")?;
                expr.validate()
            }
            BoolExpr::And(exprs) | BoolExpr::Or(exprs) => {
                if exprs.is_empty() {
                    return Err(Error::EmptyLogical {
                        logical: match self {
                            BoolExpr::And(_) => "and",
                            BoolExpr::Or(_) => "or",
                            _ => unreachable!(),
                        },
                    });
                }
                for expr in exprs {
                    expr.validate()?;
                }
                Ok(())
            }
            BoolExpr::Not(expr) => expr.validate(),
            BoolExpr::Exists(stmt) => stmt
                .validate_query_statement("EXISTS subquery must be SELECT, set, or raw statement"),
            BoolExpr::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
        }
    }
}

impl ValueExpr {
    /// Validates this value expression before SQL rendering.
    pub fn validate(&self) -> Result<()> {
        match self {
            ValueExpr::Aggregate {
                filter,
                args,
                order_by,
                over,
                distinct,
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
                if let Some(spec) = over {
                    if *distinct || !order_by.is_empty() {
                        return Err(Error::InvalidAggregateModifier { modifier: "over" });
                    }
                    spec.validate()?;
                }
                Ok(())
            }
            ValueExpr::OrderedSetAggregate {
                filter,
                args,
                within_group,
                ..
            } => {
                for arg in args {
                    arg.validate()?;
                }
                if within_group.is_empty() {
                    return Err(Error::invalid_operator(
                        "ordered_set_aggregate",
                        "within_group",
                    ));
                }
                for item in within_group {
                    item.validate()?;
                }
                if let Some(filter) = filter {
                    filter.validate()?;
                }
                Ok(())
            }
            ValueExpr::Function { args, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                Ok(())
            }
            ValueExpr::Case { branches, else_ } => {
                for (when, then) in branches {
                    when.validate()?;
                    then.validate()?;
                }
                if let Some(else_) = else_ {
                    else_.validate()?;
                }
                Ok(())
            }
            ValueExpr::Cast { expr, .. } => expr.validate(),
            ValueExpr::Binary { left, right, .. } => {
                left.validate()?;
                right.validate()
            }
            ValueExpr::Subscript { expr, index } => {
                expr.validate()?;
                index.validate()
            }
            ValueExpr::Slice { expr, start, end } => {
                expr.validate()?;
                if let Some(start) = start {
                    start.validate()?;
                }
                if let Some(end) = end {
                    end.validate()?;
                }
                Ok(())
            }
            ValueExpr::Array(values) | ValueExpr::Row(values) => {
                for value in values {
                    value.validate()?;
                }
                Ok(())
            }
            ValueExpr::Extract { expr, .. } => expr.validate(),
            ValueExpr::Window { args, spec, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                spec.validate()
            }
            ValueExpr::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
            ValueExpr::Subquery(stmt) => stmt
                .validate_query_statement("scalar subquery must be SELECT, set, or raw statement"),
            ValueExpr::InvalidAggregateModifier { expr, modifier } => {
                expr.validate()?;
                Err(Error::InvalidAggregateModifier { modifier })
            }
            ValueExpr::Field { .. }
            | ValueExpr::Excluded(_)
            | ValueExpr::Param(_)
            | ValueExpr::Null
            | ValueExpr::SqlLiteral(_)
            | ValueExpr::Keyword(_) => Ok(()),
        }
    }
}

impl super::FrameBound {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Preceding(expr) | Self::Following(expr) => expr.validate(),
            Self::UnboundedPreceding | Self::CurrentRow | Self::UnboundedFollowing => Ok(()),
        }
    }
}

impl super::WindowFrame {
    fn validate(&self) -> Result<()> {
        self.start.validate()?;
        if let Some(end) = &self.end {
            end.validate()?;
        }
        Ok(())
    }
}

impl super::WindowSpec {
    fn validate(&self) -> Result<()> {
        for expr in &self.partition_by {
            expr.validate()?;
        }
        for item in &self.order_by {
            item.validate()?;
        }
        if let Some(frame) = &self.frame {
            frame.validate()?;
        }
        Ok(())
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
    Err(Error::invalid_operator(meta.api, op.as_name()))
}

fn validate_row_compare(left: &ValueExpr, right: &ValueExpr) -> Result<()> {
    let (ValueExpr::Row(left), ValueExpr::Row(right)) = (left, right) else {
        return Ok(());
    };
    if left.len() == right.len() {
        return Ok(());
    }
    Err(Error::InvalidRowShape {
        left: left.len(),
        right: right.len(),
    })
}

fn validate_equality_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.equality {
        return Ok(());
    }
    Err(Error::invalid_operator(meta.api, operator))
}

fn validate_ordered_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.ordering {
        return Ok(());
    }
    Err(Error::invalid_operator(meta.api, operator))
}

fn validate_pattern_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.pattern {
        return Ok(());
    }
    Err(Error::invalid_operator(meta.api, operator))
}

fn validate_infix_expr(expr: &ValueExpr, op: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    let supported = match op {
        "?" | "?|" | "?&" => matches!(meta.pg, "jsonb"),
        "@>" | "<@" | "&&" | "-|-" | "<<" | ">>" | "&<" | "&>" => {
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
        "@@" => matches!(
            meta.pg,
            "text" | "varchar" | "bpchar" | "citext" | "tsvector"
        ),
        _ => true,
    };
    if supported {
        return Ok(());
    }
    Err(Error::invalid_operator(meta.api, op))
}

fn validate_array_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.pg.ends_with("[]") {
        return Ok(());
    }
    Err(Error::invalid_operator(meta.api, operator))
}
