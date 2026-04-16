use std::marker::PhantomData;

use sqlx::{Encode, Postgres, Type};

use crate::typed::{Meta, Param, SelectItem};

use super::{BoolExpr, BoolOp, ValueExpr};

/// Typed database field generated from schema metadata.
#[derive(Debug, PartialEq, Eq)]
pub struct Field<T> {
    /// Static field metadata.
    pub meta: &'static Meta,
    _ty: PhantomData<T>,
}

/// Qualified typed database field, usually created through an alias handle.
#[derive(Debug, PartialEq, Eq)]
pub struct FieldRef<T> {
    /// Static field metadata.
    pub meta: &'static Meta,
    /// Optional table/source qualifier.
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

/// Converts a field or qualified field into a [`FieldRef`].
pub trait IntoFieldRef<T> {
    /// Returns a field reference with any existing qualifier preserved.
    fn into_field_ref(self) -> FieldRef<T>;
}

impl<T> Field<T> {
    /// Creates a typed field from static metadata.
    pub const fn new(meta: &'static Meta) -> Self {
        Self {
            meta,
            _ty: PhantomData,
        }
    }

    /// Qualifies this field with a table or source alias.
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

    /// Returns this field as a value expression.
    pub fn expr(self) -> ValueExpr {
        self.reference().expr()
    }

    /// Builds a custom value operator expression for this field.
    pub fn op(self, op: &'static str, right: impl Into<ValueExpr>) -> ValueExpr {
        self.expr().op(op, right)
    }

    /// Builds a custom boolean infix predicate for this field.
    pub fn predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        self.expr().predicate(op, right)
    }

    /// Builds a negated custom boolean infix predicate for this field.
    pub fn not_predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        self.expr().not_predicate(op, right)
    }

    /// Returns this field as an aliased projection item.
    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        self.expr().alias(alias)
    }

    /// Creates an assignment for writes.
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

    /// Creates an assignment from a borrowed value by cloning it.
    pub fn set_ref<V>(self, value: &V) -> crate::typed::Assignment
    where
        V: Clone + Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.set(value.clone())
    }

    /// Returns `EXCLUDED.field` for `ON CONFLICT DO UPDATE`.
    pub fn excluded(self) -> ValueExpr {
        ValueExpr::Excluded(*self.meta)
    }

    /// Creates an assignment from `EXCLUDED.field`.
    pub fn set_excluded(self) -> crate::typed::Assignment {
        crate::typed::Assignment {
            field: *self.meta,
            value: self.excluded(),
        }
    }

    /// Builds an equality predicate against a bind value.
    pub fn eq<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Eq, value)
    }

    /// Builds an inequality predicate against a bind value.
    pub fn ne<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Ne, value)
    }

    /// Builds a greater-than predicate against a bind value.
    pub fn gt<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Gt, value)
    }

    /// Builds a greater-than-or-equal predicate against a bind value.
    pub fn gte<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Gte, value)
    }

    /// Builds a less-than predicate against a bind value.
    pub fn lt<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Lt, value)
    }

    /// Builds a less-than-or-equal predicate against a bind value.
    pub fn lte<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Lte, value)
    }

    /// Builds an `IS DISTINCT FROM` predicate against a bind value.
    pub fn is_distinct_from<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::IsDistinctFrom, value)
    }

    /// Builds an `IS NOT DISTINCT FROM` predicate against a bind value.
    pub fn is_not_distinct_from<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::IsNotDistinctFrom, value)
    }

    /// Builds `field IS NULL`.
    pub fn is_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self.expr(),
            negated: false,
        }
    }

    /// Builds `field IS NOT NULL`.
    pub fn is_not_null(self) -> BoolExpr {
        BoolExpr::IsNull {
            expr: self.expr(),
            negated: true,
        }
    }

    /// Builds `field IN (...)`.
    pub fn in_list<I, V>(self, values: I) -> BoolExpr
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.list_predicate(values, false)
    }

    /// Builds `field NOT IN (...)`.
    pub fn not_in<I, V>(self, values: I) -> BoolExpr
    where
        I: IntoIterator<Item = V>,
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.list_predicate(values, true)
    }

    /// Builds `field IN (subquery)`.
    pub fn in_subquery(self, query: impl Into<crate::typed::Stmt>) -> BoolExpr {
        BoolExpr::InSubquery {
            expr: self.expr(),
            query: Box::new(query.into()),
            negated: false,
        }
    }

    /// Builds `field NOT IN (subquery)`.
    pub fn not_in_subquery(self, query: impl Into<crate::typed::Stmt>) -> BoolExpr {
        BoolExpr::InSubquery {
            expr: self.expr(),
            query: Box::new(query.into()),
            negated: true,
        }
    }

    /// Builds `field BETWEEN low AND high`.
    pub fn between<V>(self, low: V, high: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.between_predicate(low, high, false)
    }

    /// Builds `field NOT BETWEEN low AND high`.
    pub fn not_between<V>(self, low: V, high: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.between_predicate(low, high, true)
    }

    /// Builds an equality predicate against another field of the same Rust type.
    pub fn eq_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Eq, right)
    }

    /// Builds an inequality predicate against another field of the same Rust type.
    pub fn ne_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Ne, right)
    }

    /// Builds a greater-than predicate against another field of the same Rust type.
    pub fn gt_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Gt, right)
    }

    /// Builds a greater-than-or-equal predicate against another field of the same Rust type.
    pub fn gte_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Gte, right)
    }

    /// Builds a less-than predicate against another field of the same Rust type.
    pub fn lt_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Lt, right)
    }

    /// Builds a less-than-or-equal predicate against another field of the same Rust type.
    pub fn lte_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::Lte, right)
    }

    /// Builds `IS DISTINCT FROM` against another field of the same Rust type.
    pub fn is_distinct_from_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.compare_field(BoolOp::IsDistinctFrom, right)
    }

    /// Builds `IS NOT DISTINCT FROM` against another field of the same Rust type.
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
