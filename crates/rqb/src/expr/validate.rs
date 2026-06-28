use crate::raw;
use crate::{Error, Result};

use super::{BoolExpr, BoolOp, ValueExpr};

impl BoolExpr {
    /// Validates this boolean expression before SQL rendering.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Constant(_) => Ok(()),
            Self::Compare { left, op, right } => {
                validate_compare(left, *op)?;
                validate_row_compare(left, right)?;
                left.validate()?;
                right.validate()
            }
            Self::IsNull { expr, .. } | Self::IsBoolean { expr, .. } => expr.validate(),
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
                query.validate_query_statement("IN subquery must be SELECT, set, or raw statement")
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
                validate_pattern_expr(expr, "like")?;
                expr.validate()?;
                pattern.validate()
            }
            Self::SimilarTo { expr, pattern, .. } => {
                validate_pattern_expr(expr, "similar_to")?;
                expr.validate()?;
                pattern.validate()
            }
            Self::Regex { expr, pattern, .. } => {
                validate_pattern_expr(expr, "regex")?;
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
                    return Err(Error::EmptyLogical {
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
            Self::Exists(stmt) => stmt
                .validate_query_statement("EXISTS subquery must be SELECT, set, or raw statement"),
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
        }
    }
}

impl ValueExpr {
    /// Validates this value expression before SQL rendering.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Aggregate {
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
            Self::OrderedSetAggregate {
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
            Self::Subscript { expr, index } => {
                expr.validate()?;
                index.validate()
            }
            Self::Slice { expr, start, end } => {
                expr.validate()?;
                if let Some(start) = start {
                    start.validate()?;
                }
                if let Some(end) = end {
                    end.validate()?;
                }
                Ok(())
            }
            Self::Array(values) | Self::Row(values) => {
                for value in values {
                    value.validate()?;
                }
                Ok(())
            }
            Self::Extract { expr, .. } => expr.validate(),
            Self::Window { args, spec, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                spec.validate()
            }
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
            Self::Subquery(stmt) => stmt
                .validate_query_statement("scalar subquery must be SELECT, set, or raw statement"),
            Self::InvalidAggregateModifier { expr, modifier } => {
                expr.validate()?;
                Err(Error::InvalidAggregateModifier { modifier })
            }
            Self::Field { .. }
            | Self::Excluded(_)
            | Self::Param(_)
            | Self::Null
            | Self::SqlLiteral(_)
            | Self::Keyword(_) => Ok(()),
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
