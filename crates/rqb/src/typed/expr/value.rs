use sqlx::{Encode, Postgres, Type};

use crate::typed::{Meta, OrderItem, Param, SelectItem};

use super::{BoolExpr, ValueExpr};

impl ValueExpr {
    pub fn param<T>(value: T) -> Self
    where
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        Self::Param(Param::typed(value))
    }

    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        SelectItem {
            expr: self,
            alias: Some(alias.into()),
        }
    }

    pub fn is_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self,
            negated: false,
        }
    }

    pub fn is_not_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self,
            negated: true,
        }
    }

    pub fn aggregate_order_by(mut self, item: OrderItem) -> Self {
        if let Self::Aggregate { order_by, .. } = &mut self {
            order_by.push(item);
        }
        self
    }

    pub fn aggregate_order_asc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_by(OrderItem::asc(expr))
    }

    pub fn aggregate_order_desc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_by(OrderItem::desc(expr))
    }

    pub fn order_by(self, item: OrderItem) -> Self {
        self.aggregate_order_by(item)
    }

    pub fn order_asc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_asc(expr)
    }

    pub fn order_desc(self, expr: impl Into<ValueExpr>) -> Self {
        self.aggregate_order_desc(expr)
    }

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

    pub fn filter(self, filter: BoolExpr) -> Self {
        self.aggregate_filter(filter)
    }

    pub(crate) fn field_meta(&self) -> Option<&Meta> {
        match self {
            Self::Field { meta, .. } => Some(meta),
            _ => None,
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
