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
                query.validate_query_statement(
                    "IN subquery must be SELECT, set, or raw statement",
                )?;
                let arity = match expr {
                    ValueExpr::Row(values) => values.len(),
                    _ => 1,
                };
                query.validate_projection_count(
                    arity,
                    "IN subquery column count must match its left operand",
                )
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
                left,
                op,
                right,
                checked,
                ..
            } => {
                if *checked {
                    validate_infix_expr(left, op)?;
                }
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
    pub(crate) fn prevents_row_lock(&self) -> bool {
        match self {
            Self::Aggregate { .. } | Self::OrderedSetAggregate { .. } | Self::Window { .. } => true,
            Self::Function { args, .. } | Self::Array(args) | Self::Row(args) => {
                args.iter().any(Self::prevents_row_lock)
            }
            Self::Case { branches, else_ } => {
                branches.iter().any(|(condition, value)| {
                    condition.prevents_row_lock() || value.prevents_row_lock()
                }) || else_.as_deref().is_some_and(Self::prevents_row_lock)
            }
            Self::Cast { expr, .. }
            | Self::Extract { expr, .. }
            | Self::InvalidAggregateModifier { expr, .. } => expr.prevents_row_lock(),
            Self::Binary { left, right, .. } => {
                left.prevents_row_lock() || right.prevents_row_lock()
            }
            Self::Subscript { expr, index } => {
                expr.prevents_row_lock() || index.prevents_row_lock()
            }
            Self::Slice { expr, start, end } => {
                expr.prevents_row_lock()
                    || start.as_deref().is_some_and(Self::prevents_row_lock)
                    || end.as_deref().is_some_and(Self::prevents_row_lock)
            }
            // A nested query has its own aggregate and lock scope.
            Self::Subquery(_)
            | Self::Field { .. }
            | Self::Excluded(_)
            | Self::Param(_)
            | Self::Null
            | Self::SqlLiteral(_)
            | Self::Keyword(_)
            | Self::Raw { .. } => false,
        }
    }

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
                if branches.is_empty() {
                    return Err(Error::InvalidSelectShape {
                        message: "CASE requires at least one WHEN branch",
                    });
                }
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
            ValueExpr::Subquery(stmt) => {
                stmt.validate_query_statement(
                    "scalar subquery must be SELECT, set, or raw statement",
                )?;
                stmt.validate_projection_count(1, "scalar subquery must return one column")
            }
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

impl BoolExpr {
    fn prevents_row_lock(&self) -> bool {
        match self {
            Self::Compare { left, right, .. } | Self::Infix { left, right, .. } => {
                left.prevents_row_lock() || right.prevents_row_lock()
            }
            Self::IsNull { expr, .. }
            | Self::IsBoolean { expr, .. }
            | Self::ArrayIsEmpty { expr, .. }
            | Self::InSubquery { expr, .. } => expr.prevents_row_lock(),
            Self::InList { expr, values, .. } => {
                expr.prevents_row_lock() || values.iter().any(ValueExpr::prevents_row_lock)
            }
            Self::Between {
                expr, low, high, ..
            } => expr.prevents_row_lock() || low.prevents_row_lock() || high.prevents_row_lock(),
            Self::Like { expr, pattern, .. }
            | Self::SimilarTo { expr, pattern, .. }
            | Self::Regex { expr, pattern, .. } => {
                expr.prevents_row_lock() || pattern.prevents_row_lock()
            }
            Self::Any { value, array, .. } => {
                value.prevents_row_lock() || array.prevents_row_lock()
            }
            Self::And(exprs) | Self::Or(exprs) => exprs.iter().any(Self::prevents_row_lock),
            Self::Not(expr) => expr.prevents_row_lock(),
            Self::Exists(_) | Self::Raw { .. } | Self::Constant(_) => false,
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
        use super::FrameBound;
        let rank = |bound: &FrameBound| match bound {
            FrameBound::UnboundedPreceding => 0,
            FrameBound::Preceding(_) => 1,
            FrameBound::CurrentRow => 2,
            FrameBound::Following(_) => 3,
            FrameBound::UnboundedFollowing => 4,
        };
        let end = self.end.as_ref().unwrap_or(&FrameBound::CurrentRow);
        if matches!(self.start, FrameBound::UnboundedFollowing)
            || matches!(end, FrameBound::UnboundedPreceding)
            || rank(&self.start) > rank(end)
        {
            return Err(Error::InvalidSelectShape {
                message: "invalid window frame bounds",
            });
        }
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
            let has_offset = matches!(
                frame.start,
                super::FrameBound::Preceding(_) | super::FrameBound::Following(_)
            ) || matches!(
                frame.end,
                Some(super::FrameBound::Preceding(_) | super::FrameBound::Following(_))
            );
            if (matches!(frame.kind, super::WindowFrameKind::Groups) && self.order_by.is_empty())
                || (matches!(frame.kind, super::WindowFrameKind::Range)
                    && has_offset
                    && self.order_by.len() != 1)
            {
                return Err(Error::InvalidSelectShape {
                    message: "window frame requires appropriate ORDER BY",
                });
            }
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
