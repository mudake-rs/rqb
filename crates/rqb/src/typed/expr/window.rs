use crate::typed::{OrderItem, Param};

use super::{
    FrameBound, FrameExclude, OffsetWindowFunctionBuilder, ValueExpr, WindowFrame, WindowFrameKind,
    WindowFunction, WindowFunctionBuilder, WindowSpec,
};

impl WindowFunction {
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
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Rows => "ROWS",
            Self::Range => "RANGE",
            Self::Groups => "GROUPS",
        }
    }
}

impl FrameExclude {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::CurrentRow => "EXCLUDE CURRENT ROW",
            Self::Group => "EXCLUDE GROUP",
            Self::Ties => "EXCLUDE TIES",
            Self::NoOthers => "EXCLUDE NO OTHERS",
        }
    }
}

impl WindowFrame {
    pub fn new(kind: WindowFrameKind, start: FrameBound) -> Self {
        Self {
            kind,
            start,
            end: None,
            exclude: None,
        }
    }

    pub fn between(mut self, end: FrameBound) -> Self {
        self.end = Some(end);
        self
    }

    pub fn exclude(mut self, exclude: FrameExclude) -> Self {
        self.exclude = Some(exclude);
        self
    }
}

impl WindowSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn partition_by(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.partition_by.push(expr.into());
        self
    }

    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order_by.push(item);
        self
    }

    pub fn order_asc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::asc(expr));
        self
    }

    pub fn order_desc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::desc(expr));
        self
    }

    pub fn order_asc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::asc_nulls_first(expr));
        self
    }

    pub fn order_asc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::asc_nulls_last(expr));
        self
    }

    pub fn order_desc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::desc_nulls_first(expr));
        self
    }

    pub fn order_desc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order_by.push(OrderItem::desc_nulls_last(expr));
        self
    }

    pub fn frame(mut self, frame: WindowFrame) -> Self {
        self.frame = Some(Box::new(frame));
        self
    }

    pub fn rows(self, start: FrameBound) -> Self {
        self.frame(WindowFrame::new(WindowFrameKind::Rows, start))
    }

    pub fn range(self, start: FrameBound) -> Self {
        self.frame(WindowFrame::new(WindowFrameKind::Range, start))
    }

    pub fn groups(self, start: FrameBound) -> Self {
        self.frame(WindowFrame::new(WindowFrameKind::Groups, start))
    }
}

impl WindowFunctionBuilder {
    pub fn over(self, spec: WindowSpec) -> ValueExpr {
        ValueExpr::Window {
            function: self.function,
            args: self.args,
            spec,
        }
    }
}

impl OffsetWindowFunctionBuilder {
    pub fn offset(mut self, offset: impl Into<ValueExpr>) -> Self {
        self.offset = Some(offset.into());
        self
    }

    pub fn default(mut self, value: impl Into<ValueExpr>) -> Self {
        self.default = Some(value.into());
        self
    }

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

pub fn window() -> WindowSpec {
    WindowSpec::new()
}

pub fn partition_by(expr: impl Into<ValueExpr>) -> WindowSpec {
    WindowSpec::new().partition_by(expr)
}

pub fn row_number() -> WindowFunctionBuilder {
    window_function(WindowFunction::RowNumber, [])
}

pub fn rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::Rank, [])
}

pub fn dense_rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::DenseRank, [])
}

pub fn first_value(expr: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::FirstValue, [expr.into()])
}

pub fn last_value(expr: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::LastValue, [expr.into()])
}

pub fn nth_value(expr: impl Into<ValueExpr>, nth: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::NthValue, [expr.into(), nth.into()])
}

pub fn ntile(buckets: impl Into<ValueExpr>) -> WindowFunctionBuilder {
    window_function(WindowFunction::Ntile, [buckets.into()])
}

pub fn percent_rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::PercentRank, [])
}

pub fn cume_dist() -> WindowFunctionBuilder {
    window_function(WindowFunction::CumeDist, [])
}

pub fn lag(expr: impl Into<ValueExpr>) -> OffsetWindowFunctionBuilder {
    offset_window_function(WindowFunction::Lag, expr)
}

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

pub fn rows(start: FrameBound) -> WindowFrame {
    WindowFrame::new(WindowFrameKind::Rows, start)
}

pub fn range(start: FrameBound) -> WindowFrame {
    WindowFrame::new(WindowFrameKind::Range, start)
}

pub fn groups(start: FrameBound) -> WindowFrame {
    WindowFrame::new(WindowFrameKind::Groups, start)
}

pub fn unbounded_preceding() -> FrameBound {
    FrameBound::UnboundedPreceding
}

pub fn preceding(expr: impl Into<ValueExpr>) -> FrameBound {
    FrameBound::Preceding(Box::new(expr.into()))
}

pub fn current_row() -> FrameBound {
    FrameBound::CurrentRow
}

pub fn following(expr: impl Into<ValueExpr>) -> FrameBound {
    FrameBound::Following(Box::new(expr.into()))
}

pub fn unbounded_following() -> FrameBound {
    FrameBound::UnboundedFollowing
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
