use crate::{BindValue, Meta, OrderItem, Param, SelectItem};

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
    pub fn end(self) -> ValueExpr {
        ValueExpr::Case {
            branches: self.branches,
            else_: None,
        }
    }

    /// Finishes the expression with an `ELSE` branch.
    pub fn else_(self, value: impl Into<ValueExpr>) -> ValueExpr {
        ValueExpr::Case {
            branches: self.branches,
            else_: Some(Box::new(value.into())),
        }
    }
}

impl Meta {
    /// Returns this metadata as an unqualified value expression.
    ///
    /// This is useful for raw-only extension columns that are known to the
    /// schema generator but intentionally do not have a typed [`Field`](crate::Field).
    pub fn expr(self) -> ValueExpr {
        ValueExpr::Field {
            meta: self,
            qualifier: None,
        }
    }

    /// Returns this metadata as a qualified value expression.
    pub fn at(self, qualifier: impl Into<String>) -> ValueExpr {
        ValueExpr::Field {
            meta: self,
            qualifier: Some(qualifier.into()),
        }
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

    /// Returns this expression as an aliased projection item.
    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        SelectItem {
            expr: self,
            alias: Some(alias.into()),
        }
    }

    /// Casts this expression to a Postgres type.
    pub fn cast(self, pg: &'static str) -> Self {
        Self::Cast {
            expr: Box::new(self),
            pg,
        }
    }

    /// Builds a custom value operator expression.
    ///
    /// Use this as the typed escape hatch for extension operators such as
    /// pgvector distance operators.
    pub fn op(self, op: &'static str, right: impl Into<ValueExpr>) -> Self {
        Self::Binary {
            left: Box::new(self),
            op: ValueOp::Custom(op),
            right: Box::new(right.into()),
        }
    }

    /// Builds a custom boolean infix predicate.
    pub fn predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::Infix {
            left: self,
            op,
            right: right.into(),
            negated: false,
        }
    }

    /// Builds a negated custom boolean infix predicate.
    pub fn not_predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::Infix {
            left: self,
            op,
            right: right.into(),
            negated: true,
        }
    }

    /// Builds `expr IS NULL`.
    pub fn is_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self,
            negated: false,
        }
    }

    /// Builds `expr IS NOT NULL`.
    pub fn is_not_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self,
            negated: true,
        }
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
    pub fn aggregate_order_by(mut self, item: OrderItem) -> Self {
        if let Self::Aggregate { order_by, .. } = &mut self {
            order_by.push(item);
        }
        self
    }

    /// Adds aggregate-local ascending order.
    pub fn aggregate_order_asc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_by(OrderItem::asc(expr))
    }

    /// Adds aggregate-local descending order.
    pub fn aggregate_order_desc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_by(OrderItem::desc(expr))
    }

    /// Alias for [`ValueExpr::aggregate_order_by`].
    pub fn order_by(self, item: OrderItem) -> Self {
        self.aggregate_order_by(item)
    }

    /// Alias for [`ValueExpr::aggregate_order_asc`].
    pub fn order_asc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_asc(expr)
    }

    /// Alias for [`ValueExpr::aggregate_order_desc`].
    pub fn order_desc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_desc(expr)
    }

    /// Adds an aggregate `FILTER (WHERE ...)` predicate.
    pub fn aggregate_filter(mut self, filter: BoolExpr) -> Self {
        match &mut self {
            Self::Aggregate {
                filter: current, ..
            }
            | Self::OrderedSetAggregate {
                filter: current, ..
            } => {
                *current = Some(Box::new(BoolExpr::and_option(
                    current.take().map(|existing| *existing),
                    filter,
                )));
            }
            _ => {}
        }
        self
    }

    /// Alias for [`ValueExpr::aggregate_filter`].
    pub fn filter(self, filter: BoolExpr) -> Self {
        self.aggregate_filter(filter)
    }

    pub(crate) fn field_meta(&self) -> Option<&Meta> {
        match self {
            Self::Field { meta, .. } => Some(meta),
            _ => None,
        }
    }

    fn compare(self, op: BoolOp, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::Compare {
            left: self,
            op,
            right: right.into(),
        }
    }
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
