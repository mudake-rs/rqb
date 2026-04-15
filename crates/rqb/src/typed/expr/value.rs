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

    pub fn aggregate_filter(mut self, filter: BoolExpr) -> Self {
        if let Self::Aggregate {
            filter: current, ..
        } = &mut self
        {
            *current = Some(Box::new(match current.take() {
                Some(existing) => BoolExpr::And(vec![*existing, filter]),
                None => filter,
            }));
        }
        self
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
