use crate::typed::{OrderItem, Param};

use super::{
    OffsetWindowFunctionBuilder, ValueExpr, WindowFunction, WindowFunctionBuilder, WindowSpec,
};

impl WindowFunction {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::RowNumber => "row_number",
            Self::Rank => "rank",
            Self::DenseRank => "dense_rank",
            Self::Lag => "lag",
            Self::Lead => "lead",
        }
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

pub fn count_all() -> ValueExpr {
    aggregate("count", [], false)
}

pub fn count(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("count", [expr.into()], false)
}

pub fn count_distinct(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("count", [expr.into()], true)
}

pub fn sum(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("sum", [expr.into()], false)
}

pub fn avg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("avg", [expr.into()], false)
}

pub fn min(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("min", [expr.into()], false)
}

pub fn max(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("max", [expr.into()], false)
}

pub fn array_agg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("array_agg", [expr.into()], false)
}

pub fn array_agg_distinct(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("array_agg", [expr.into()], true)
}

pub fn json_agg(expr: impl Into<ValueExpr>) -> ValueExpr {
    aggregate("json_agg", [expr.into()], false)
}

pub fn string_agg(expr: impl Into<ValueExpr>, separator: impl Into<String>) -> ValueExpr {
    aggregate(
        "string_agg",
        [
            expr.into(),
            ValueExpr::Param(Param::typed(separator.into())),
        ],
        false,
    )
}

pub fn aggregate(
    name: &'static str,
    args: impl IntoIterator<Item = ValueExpr>,
    distinct: bool,
) -> ValueExpr {
    ValueExpr::Aggregate {
        name,
        args: args.into_iter().collect(),
        distinct,
        order_by: Vec::new(),
        filter: None,
    }
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
