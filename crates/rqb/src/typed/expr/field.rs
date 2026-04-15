use std::marker::PhantomData;

use sqlx::{Encode, Postgres, Type};

use crate::typed::{Meta, Param, SelectItem};

use super::{BoolExpr, BoolOp, ValueExpr};

#[derive(Debug, PartialEq, Eq)]
pub struct Field<T> {
    pub meta: &'static Meta,
    _ty: PhantomData<T>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FieldRef<T> {
    pub meta: &'static Meta,
    pub qualifier: Option<String>,
    _ty: PhantomData<T>,
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Field<T> {}

impl<T> Clone for FieldRef<T> {
    fn clone(&self) -> Self {
        Self {
            meta: self.meta,
            qualifier: self.qualifier.clone(),
            _ty: PhantomData,
        }
    }
}

pub trait IntoFieldRef<T> {
    fn into_field_ref(self) -> FieldRef<T>;
}

impl<T> Field<T> {
    pub const fn new(meta: &'static Meta) -> Self {
        Self {
            meta,
            _ty: PhantomData,
        }
    }

    pub fn at(self, qualifier: impl Into<String>) -> FieldRef<T> {
        FieldRef {
            meta: self.meta,
            qualifier: Some(qualifier.into()),
            _ty: PhantomData,
        }
    }

    pub(super) fn reference(self) -> FieldRef<T> {
        FieldRef {
            meta: self.meta,
            qualifier: None,
            _ty: PhantomData,
        }
    }

    pub fn expr(self) -> ValueExpr {
        self.reference().expr()
    }

    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        self.expr().alias(alias)
    }

    pub fn set<V>(self, value: V) -> crate::typed::Assignment
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        crate::typed::Assignment {
            field: *self.meta,
            value: ValueExpr::Param(Param::typed(value.into())),
        }
    }

    pub fn set_ref<V>(self, value: &V) -> crate::typed::Assignment
    where
        V: Clone + Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.set(value.clone())
    }

    pub fn excluded(self) -> ValueExpr {
        ValueExpr::Excluded(*self.meta)
    }

    pub fn set_excluded(self) -> crate::typed::Assignment {
        crate::typed::Assignment {
            field: *self.meta,
            value: self.excluded(),
        }
    }

    pub fn eq<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Eq, value)
    }

    pub fn ne<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Ne, value)
    }

    pub fn gt<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Gt, value)
    }

    pub fn gte<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Gte, value)
    }

    pub fn lt<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Lt, value)
    }

    pub fn lte<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Lte, value)
    }

    pub fn is_distinct_from<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::IsDistinctFrom, value)
    }

    pub fn is_not_distinct_from<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::IsNotDistinctFrom, value)
    }

    pub fn is_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self.expr(),
            negated: false,
        }
    }

    pub fn is_not_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self.expr(),
            negated: true,
        }
    }

    pub fn in_list<I, V>(self, values: I) -> BoolExpr
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.list_predicate(values, false)
    }

    pub fn not_in<I, V>(self, values: I) -> BoolExpr
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.list_predicate(values, true)
    }

    pub fn in_subquery(self, query: impl Into<crate::typed::Stmt>) -> BoolExpr {
        BoolExpr::InSubquery {
            expr: self.expr(),
            query: Box::new(query.into()),
            negated: false,
        }
    }

    pub fn not_in_subquery(self, query: impl Into<crate::typed::Stmt>) -> BoolExpr {
        BoolExpr::InSubquery {
            expr: self.expr(),
            query: Box::new(query.into()),
            negated: true,
        }
    }

    pub fn between<V>(self, low: V, high: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.between_predicate(low, high, false)
    }

    pub fn not_between<V>(self, low: V, high: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.between_predicate(low, high, true)
    }

    pub fn eq_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Eq, right)
    }

    pub fn ne_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Ne, right)
    }

    pub fn gt_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Gt, right)
    }

    pub fn gte_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Gte, right)
    }

    pub fn lt_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Lt, right)
    }

    pub fn lte_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Lte, right)
    }

    pub fn is_distinct_from_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::IsDistinctFrom, right)
    }

    pub fn is_not_distinct_from_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::IsNotDistinctFrom, right)
    }

    fn compare<V>(self, op: BoolOp, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        BoolExpr::Compare {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(value.into())),
        }
    }

    fn compare_field<R>(self, op: BoolOp, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        BoolExpr::Compare {
            left: self.expr(),
            op,
            right: right.into_field_ref().expr(),
        }
    }

    fn list_predicate<I, V>(self, values: I, negated: bool) -> BoolExpr
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        let values = values
            .into_iter()
            .map(|value| ValueExpr::Param(Param::typed(value.into())))
            .collect::<Vec<_>>();
        if values.is_empty() {
            return BoolExpr::Constant(negated);
        }
        BoolExpr::InList {
            expr: self.expr(),
            values,
            negated,
        }
    }

    fn between_predicate<V>(self, low: V, high: V, negated: bool) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        BoolExpr::Between {
            expr: self.expr(),
            low: ValueExpr::Param(Param::typed(low.into())),
            high: ValueExpr::Param(Param::typed(high.into())),
            negated,
        }
    }
}

impl<T> IntoFieldRef<T> for Field<T> {
    fn into_field_ref(self) -> FieldRef<T> {
        self.reference()
    }
}

impl<T> IntoFieldRef<T> for FieldRef<T> {
    fn into_field_ref(self) -> FieldRef<T> {
        self
    }
}

impl<T> From<Field<T>> for ValueExpr {
    fn from(field: Field<T>) -> Self {
        field.expr()
    }
}

impl<T> From<FieldRef<T>> for ValueExpr {
    fn from(field: FieldRef<T>) -> Self {
        field.expr()
    }
}
