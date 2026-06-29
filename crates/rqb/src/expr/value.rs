use crate::{BindValue, Meta, OrderItem, Param, WindowSpec};

use super::{BoolExpr, BoolOp, CaseBuilder, ValueExpr, ValueOp};

impl CaseBuilder {
    /// Creates an empty `CASE` expression builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a `WHEN condition THEN value` branch.
    pub fn when(mut self, condition: BoolExpr, value: impl Into<ValueExpr>) -> Self {
        self.branches.push((condition, value.into()));
        self
    }

    /// Finishes the expression without an `ELSE` branch.
    #[inline]
    pub fn end(self) -> ValueExpr {
        ValueExpr::case(self.branches, None)
    }

    /// Finishes the expression with an `ELSE` branch.
    pub fn else_(self, value: impl Into<ValueExpr>) -> ValueExpr {
        ValueExpr::case(self.branches, Some(Box::new(value.into())))
    }
}

impl Meta {
    /// Returns this metadata as an unqualified value expression.
    ///
    /// This is useful for raw-only extension columns that are known to the
    /// schema generator but intentionally do not have a typed [`Field`](crate::Field).
    #[inline]
    pub fn expr(self) -> ValueExpr {
        ValueExpr::field(self, None)
    }

    /// Returns this metadata as a qualified value expression.
    pub fn at(self, qualifier: impl Into<String>) -> ValueExpr {
        ValueExpr::field(self, Some(qualifier.into()))
    }
}

impl From<Meta> for ValueExpr {
    fn from(meta: Meta) -> Self {
        meta.expr()
    }
}

impl From<&Meta> for ValueExpr {
    fn from(meta: &Meta) -> Self {
        (*meta).expr()
    }
}

impl ValueExpr {
    /// Wraps a typed Rust value as a SQL bind parameter expression.
    pub fn param<T>(value: T) -> Self
    where
        T: BindValue,
    {
        Self::Param(Param::typed(value))
    }

    /// Casts this expression to a Postgres type.
    #[inline]
    pub fn cast(self, pg: &'static str) -> Self {
        Self::cast_expr(self, pg)
    }

    /// Builds a custom value operator expression.
    ///
    /// Use this as the typed escape hatch for extension operators such as
    /// pgvector distance operators.
    pub fn op(self, op: &'static str, right: impl Into<ValueExpr>) -> Self {
        Self::binary(self, ValueOp::Custom(op), right.into())
    }

    /// Builds a custom boolean infix predicate.
    pub fn predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::infix(self, op, right.into(), false)
    }

    /// Builds a negated custom boolean infix predicate.
    pub fn not_predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::infix(self, op, right.into(), true)
    }

    /// Builds `expr IS NULL`.
    #[inline]
    pub fn is_null(self) -> BoolExpr {
        BoolExpr::is_null_expr(self, false)
    }

    /// Builds `expr IS NOT NULL`.
    #[inline]
    pub fn is_not_null(self) -> BoolExpr {
        BoolExpr::is_null_expr(self, true)
    }

    /// Compares this expression with another expression or bind value.
    pub fn eq(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::Eq, right)
    }

    /// Compares this expression with another expression or bind value.
    pub fn ne(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::Ne, right)
    }

    /// Compares this expression with another expression or bind value.
    pub fn gt(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::Gt, right)
    }

    /// Compares this expression with another expression or bind value.
    pub fn gte(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::Gte, right)
    }

    /// Compares this expression with another expression or bind value.
    pub fn lt(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::Lt, right)
    }

    /// Compares this expression with another expression or bind value.
    pub fn lte(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::Lte, right)
    }

    /// Builds `IS DISTINCT FROM` for null-safe comparison.
    pub fn is_distinct_from(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::IsDistinctFrom, right)
    }

    /// Builds `IS NOT DISTINCT FROM` for null-safe comparison.
    pub fn is_not_distinct_from(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare(BoolOp::IsNotDistinctFrom, right)
    }

    /// Adds aggregate-local `ORDER BY`.
    #[inline]
    pub fn aggregate_order_by(self, item: OrderItem) -> Self {
        match self {
            ValueExpr::Aggregate {
                name,
                args,
                distinct,
                mut order_by,
                filter,
                over: None,
            } => {
                order_by.push(item);
                Self::aggregate(name, args, distinct, order_by, filter, None)
            }
            expr => Self::invalid_aggregate_modifier(expr, "aggregate_order_by"),
        }
    }

    /// Adds aggregate-local ascending order.
    pub fn aggregate_order_asc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_by(OrderItem::asc(expr))
    }

    /// Adds aggregate-local descending order.
    pub fn aggregate_order_desc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_by(OrderItem::desc(expr))
    }

    /// Adds an aggregate `FILTER (WHERE ...)` predicate.
    #[inline]
    pub fn aggregate_filter(self, next_filter: BoolExpr) -> Self {
        match self {
            ValueExpr::Aggregate {
                name,
                args,
                distinct,
                order_by,
                filter,
                over,
            } => Self::aggregate(
                name,
                args,
                distinct,
                order_by,
                combine_aggregate_filters(filter, next_filter),
                over,
            ),
            ValueExpr::OrderedSetAggregate {
                name,
                args,
                within_group,
                filter,
            } => Self::ordered_set_aggregate(
                name,
                args,
                within_group,
                combine_aggregate_filters(filter, next_filter),
            ),
            expr => Self::invalid_aggregate_modifier(expr, "aggregate_filter"),
        }
    }

    /// Adds `OVER (...)` to an aggregate function call.
    ///
    /// PostgreSQL aggregate window calls use `ORDER BY` inside `OVER (...)`;
    /// aggregate-local `DISTINCT` and `ORDER BY` are rejected for this modifier.
    #[inline]
    pub fn over(self, spec: WindowSpec) -> Self {
        match self {
            ValueExpr::Aggregate {
                name,
                args,
                distinct,
                order_by,
                filter,
                over,
            } => {
                if distinct || !order_by.is_empty() {
                    return Self::invalid_aggregate_modifier(
                        Self::aggregate(name, args, distinct, order_by, filter, over),
                        "over",
                    );
                }
                Self::aggregate(name, args, distinct, order_by, filter, Some(Box::new(spec)))
            }
            expr => Self::invalid_aggregate_modifier(expr, "over"),
        }
    }

    pub(crate) fn field_meta(&self) -> Option<&Meta> {
        match self {
            ValueExpr::Field { meta, .. } => Some(meta),
            _ => None,
        }
    }

    fn compare(self, op: BoolOp, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::compare(self, op, right.into())
    }
}

fn combine_aggregate_filters(
    current: Option<Box<BoolExpr>>,
    next: BoolExpr,
) -> Option<Box<BoolExpr>> {
    Some(Box::new(BoolExpr::and_option(
        current.map(|existing| *existing),
        next,
    )))
}

impl From<Param> for ValueExpr {
    fn from(param: Param) -> Self {
        Self::Param(param)
    }
}

impl From<String> for ValueExpr {
    fn from(value: String) -> Self {
        Self::Param(Param::typed(value))
    }
}

impl From<&str> for ValueExpr {
    fn from(value: &str) -> Self {
        Self::Param(Param::typed(value.to_owned()))
    }
}

macro_rules! impl_param_value_expr {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for ValueExpr {
                fn from(value: $ty) -> Self {
                    Self::Param(Param::typed(value))
                }
            }
        )*
    };
}

impl_param_value_expr!(bool, i16, i32, i64, f32, f64);
impl_param_value_expr!(
    Vec<u8>,
    uuid::Uuid,
    std::time::Duration,
    sqlx::postgres::types::PgInterval,
    sqlx::types::BigDecimal,
    chrono::Duration,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::FixedOffset>,
    chrono::NaiveDate,
    chrono::NaiveDateTime,
    chrono::NaiveTime,
    serde_json::Value,
);
