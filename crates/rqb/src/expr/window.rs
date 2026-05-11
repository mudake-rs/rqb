use crate::{OrderItem, Param};

use super::{
    FrameBound, FrameExclude, OffsetWindowFunctionBuilder, ValueExpr, WindowFrame, WindowFrameKind,
    WindowFunction, WindowFunctionBuilder, WindowSpec,
};

impl WindowFunction {
    /// Returns the SQL function name.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::RowNumber => "row_number",
            Self::Rank => "rank",
            Self::DenseRank => "dense_rank",
            Self::Lag => "lag",
            Self::Lead => "lead",
            Self::FirstValue => "first_value",
            Self::LastValue => "last_value",
            Self::NthValue => "nth_value",
            Self::Ntile => "ntile",
            Self::PercentRank => "percent_rank",
            Self::CumeDist => "cume_dist",
        }
    }
}

impl WindowFrameKind {
    /// Returns the SQL frame unit.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Rows => "ROWS",
            Self::Range => "RANGE",
            Self::Groups => "GROUPS",
        }
    }
}

impl FrameExclude {
    /// Returns the SQL exclusion clause.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::CurrentRow => "EXCLUDE CURRENT ROW",
            Self::Group => "EXCLUDE GROUP",
            Self::Ties => "EXCLUDE TIES",
            Self::NoOthers => "EXCLUDE NO OTHERS",
        }
    }
}

impl FrameBound {
    /// Returns `UNBOUNDED PRECEDING`.
    #[inline]
    pub const fn unbounded_preceding() -> Self {
        Self::UnboundedPreceding
    }

    /// Returns `expr PRECEDING`.
    pub fn preceding(expr: impl Into<ValueExpr>) -> Self {
        Self::Preceding(Box::new(expr.into()))
    }

    /// Returns `CURRENT ROW`.
    #[inline]
    pub const fn current_row() -> Self {
        Self::CurrentRow
    }

    /// Returns `expr FOLLOWING`.
    pub fn following(expr: impl Into<ValueExpr>) -> Self {
        Self::Following(Box::new(expr.into()))
    }

    /// Returns `UNBOUNDED FOLLOWING`.
    #[inline]
    pub const fn unbounded_following() -> Self {
        Self::UnboundedFollowing
    }
}

impl WindowFrame {
    /// Creates a window frame with a start boundary.
    pub fn new(kind: WindowFrameKind, start: FrameBound) -> Self {
        Self {
            kind,
            start,
            end: None,
            exclude: None,
        }
    }

    /// Adds an end boundary, rendering `BETWEEN start AND end`.
    pub fn between(mut self, end: FrameBound) -> Self {
        self.end = Some(end);
        self
    }

    /// Adds an exclusion clause.
    pub fn exclude(mut self, exclude: FrameExclude) -> Self {
        self.exclude = Some(exclude);
        self
    }
}

impl WindowSpec {
    /// Creates an empty window specification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a `PARTITION BY` expression.
    pub fn partition_by(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.partition_by.push(expr.into());
        self
    }

    /// Adds a fully specified `ORDER BY` item.
    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order_by.push(item);
        self
    }

    /// Adds ascending order.
    pub fn order_asc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::asc(expr));
        self
    }

    /// Adds descending order.
    pub fn order_desc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::desc(expr));
        self
    }

    /// Adds ascending order with `NULLS FIRST`.
    pub fn order_asc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::asc_nulls_first(expr));
        self
    }

    /// Adds ascending order with `NULLS LAST`.
    pub fn order_asc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::asc_nulls_last(expr));
        self
    }

    /// Adds descending order with `NULLS FIRST`.
    pub fn order_desc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::desc_nulls_first(expr));
        self
    }

    /// Adds descending order with `NULLS LAST`.
    pub fn order_desc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::desc_nulls_last(expr));
        self
    }

    /// Sets the window frame.
    pub fn frame(mut self, frame: WindowFrame) -> Self {
        self.frame = Some(Box::new(frame));
        self
    }

    /// Sets a `ROWS` frame.
    pub fn rows(self, start: FrameBound) -> Self {
        self.frame(WindowFrame::new(WindowFrameKind::Rows, start))
    }

    /// Sets a `RANGE` frame.
    pub fn range(self, start: FrameBound) -> Self {
        self.frame(WindowFrame::new(WindowFrameKind::Range, start))
    }

    /// Sets a `GROUPS` frame.
    pub fn groups(self, start: FrameBound) -> Self {
        self.frame(WindowFrame::new(WindowFrameKind::Groups, start))
    }
}

impl WindowFunctionBuilder {
    /// Attaches an `OVER (...)` clause and returns a value expression.
    pub fn over(self, spec: WindowSpec) -> ValueExpr {
        ValueExpr::Window {
            function: self.function,
            args: self.args,
            spec,
        }
    }
}

impl OffsetWindowFunctionBuilder {
    /// Sets the `lag` / `lead` offset argument.
    pub fn offset(mut self, offset: impl Into<ValueExpr>) -> Self {
        self.offset = Some(offset.into());
        self
    }

    /// Sets the `lag` / `lead` default argument.
    pub fn default(mut self, value: impl Into<ValueExpr>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Attaches an `OVER (...)` clause and returns a value expression.
    pub fn over(self, spec: WindowSpec) -> ValueExpr {
        let mut args = vec![self.value];
        match (self.offset, self.default) {
            (Some(offset), Some(default)) => {
                args.push(offset);
                args.push(default);
            }
            (Some(offset), None) => args.push(offset),
            (None, Some(default)) => {
                // PostgreSQL requires the offset argument before a default value.
                // lag/lead default to offset 1 when the caller only sets default.
                args.push(ValueExpr::Param(Param::typed(1_i32)));
                args.push(default);
            }
            (None, None) => {}
        }
        ValueExpr::Window {
            function: self.function,
            args,
            spec,
        }
    }
}

/// Starts a window specification.
#[inline]
pub fn window() -> WindowSpec {
    WindowSpec::new()
}

/// Starts a window specification with one partition expression.
pub fn partition_by(expr: impl Into<ValueExpr>) -> WindowSpec {
    WindowSpec::new().partition_by(expr)
}

/// Builds `row_number()`.
#[inline]
pub fn row_number() -> WindowFunctionBuilder {
    window_function(WindowFunction::RowNumber, [])
}

/// Builds `rank()`.
#[inline]
pub fn rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::Rank, [])
}

/// Builds `dense_rank()`.
#[inline]
pub fn dense_rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::DenseRank, [])
}

/// Builds `first_value(expr)`.
pub fn first_value(expr: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::FirstValue, [expr.into()])
}

/// Builds `last_value(expr)`.
pub fn last_value(expr: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::LastValue, [expr.into()])
}

/// Builds `nth_value(expr, nth)`.
pub fn nth_value(expr: impl Into<ValueExpr>, nth: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::NthValue, [expr.into(), nth.into()])
}

/// Builds `ntile(buckets)`.
pub fn ntile(buckets: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::Ntile, [buckets.into()])
}

/// Builds `percent_rank()`.
#[inline]
pub fn percent_rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::PercentRank, [])
}

/// Builds `cume_dist()`.
#[inline]
pub fn cume_dist() -> WindowFunctionBuilder {
    window_function(WindowFunction::CumeDist, [])
}

/// Builds `lag(expr)`.
pub fn lag(expr: impl Into<ValueExpr>) -> OffsetWindowFunctionBuilder {
    offset_window_function(WindowFunction::Lag, expr)
}

/// Builds `lead(expr)`.
pub fn lead(expr: impl Into<ValueExpr>) -> OffsetWindowFunctionBuilder {
    offset_window_function(WindowFunction::Lead, expr)
}

fn window_function<I>(function: WindowFunction, args: I) -> WindowFunctionBuilder
where
    I: IntoIterator<Item = ValueExpr>,
{
    WindowFunctionBuilder {
        function,
        args: args.into_iter().collect(),
    }
}

/// Starts a `ROWS` frame.
pub fn rows(start: FrameBound) -> WindowFrame {
    WindowFrame::new(WindowFrameKind::Rows, start)
}

/// Starts a `RANGE` frame.
pub fn range(start: FrameBound) -> WindowFrame {
    WindowFrame::new(WindowFrameKind::Range, start)
}

/// Starts a `GROUPS` frame.
pub fn groups(start: FrameBound) -> WindowFrame {
    WindowFrame::new(WindowFrameKind::Groups, start)
}

/// Returns `UNBOUNDED PRECEDING`.
#[inline]
pub fn unbounded_preceding() -> FrameBound {
    FrameBound::unbounded_preceding()
}

/// Returns `expr PRECEDING`.
pub fn preceding(expr: impl Into<ValueExpr>) -> FrameBound {
    FrameBound::preceding(expr)
}

/// Returns `CURRENT ROW`.
#[inline]
pub fn current_row() -> FrameBound {
    FrameBound::current_row()
}

/// Returns `expr FOLLOWING`.
pub fn following(expr: impl Into<ValueExpr>) -> FrameBound {
    FrameBound::following(expr)
}

/// Returns `UNBOUNDED FOLLOWING`.
#[inline]
pub fn unbounded_following() -> FrameBound {
    FrameBound::unbounded_following()
}

fn offset_window_function(
    function: WindowFunction,
    expr: impl Into<ValueExpr>,
) -> OffsetWindowFunctionBuilder {
    OffsetWindowFunctionBuilder {
        function,
        value: expr.into(),
        offset: None,
        default: None,
    }
}
