use std::marker::PhantomData;

use crate::{BindValue, Meta, Param, SelectItem};

use super::{BoolExpr, BoolOp, ValueExpr};

/// Typed database field generated from schema metadata.
#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub struct Field<T> {
    /// Static field metadata.
    pub meta: &'static Meta,
    _ty: PhantomData<T>,
}

/// Qualified typed database field, usually created through an alias handle.
#[derive(Debug, PartialEq, Eq)]
#[must_use]
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

    /// Returns this field qualified as `old.field` for PostgreSQL 18 DML `RETURNING`.
    pub fn old_value(self) -> FieldRef<T> {
        self.at("old")
    }

    /// Returns this field qualified as `new.field` for PostgreSQL 18 DML `RETURNING`.
    pub fn new_value(self) -> FieldRef<T> {
        self.at("new")
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

    /// Returns `EXCLUDED.field` for `ON CONFLICT DO UPDATE`.
    pub fn excluded(self) -> ValueExpr {
        ValueExpr::Excluded(*self.meta)
    }

    /// Creates an assignment from an expression.
    pub fn set_expr(self, value: impl Into<ValueExpr>) -> crate::Assignment {
        crate::Assignment {
            field: *self.meta,
            value: value.into(),
        }
    }

    /// Creates an assignment that writes SQL `NULL`.
    pub fn set_null(self) -> crate::Assignment {
        self.set_expr(ValueExpr::Null)
    }

    /// Creates an assignment from `EXCLUDED.field`.
    pub fn set_excluded(self) -> crate::Assignment {
        crate::Assignment {
            field: *self.meta,
            value: self.excluded(),
        }
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

    /// Builds `field IN (subquery)`.
    pub fn in_subquery(self, query: impl Into<crate::Stmt>) -> BoolExpr {
        BoolExpr::InSubquery {
            expr: self.expr(),
            query: Box::new(query.into()),
            negated: false,
        }
    }

    /// Builds `field NOT IN (subquery)`.
    pub fn not_in_subquery(self, query: impl Into<crate::Stmt>) -> BoolExpr {
        BoolExpr::InSubquery {
            expr: self.expr(),
            query: Box::new(query.into()),
            negated: true,
        }
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

    /// Builds an equality predicate against a value expression.
    pub fn eq_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::Eq, right)
    }

    /// Builds an inequality predicate against a value expression.
    pub fn ne_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::Ne, right)
    }

    /// Builds a greater-than predicate against a value expression.
    pub fn gt_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::Gt, right)
    }

    /// Builds a greater-than-or-equal predicate against a value expression.
    pub fn gte_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::Gte, right)
    }

    /// Builds a less-than predicate against a value expression.
    pub fn lt_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::Lt, right)
    }

    /// Builds a less-than-or-equal predicate against a value expression.
    pub fn lte_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::Lte, right)
    }

    /// Builds `IS DISTINCT FROM` against a value expression.
    pub fn is_distinct_from_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::IsDistinctFrom, right)
    }

    /// Builds `IS NOT DISTINCT FROM` against a value expression.
    pub fn is_not_distinct_from_expr(self, right: impl Into<ValueExpr>) -> BoolExpr {
        self.compare_expr(BoolOp::IsNotDistinctFrom, right)
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

    fn compare_expr(self, op: BoolOp, right: impl Into<ValueExpr>) -> BoolExpr {
        BoolExpr::Compare {
            left: self.expr(),
            op,
            right: right.into(),
        }
    }
}

impl<T: BindValue> Field<T> {
    /// Creates an assignment for writes.
    pub fn set(self, value: impl Into<T>) -> crate::Assignment {
        self.set_expr(ValueExpr::Param(Param::typed(value.into())))
    }

    /// Creates an assignment from a borrowed value by cloning it.
    pub fn set_ref<V>(self, value: &V) -> crate::Assignment
    where
        V: Clone + Into<T>,
    {
        self.set(value.clone())
    }

    /// Builds an equality predicate against a bind value.
    pub fn eq(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::Eq, value)
    }

    /// Builds an inequality predicate against a bind value.
    pub fn ne(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::Ne, value)
    }

    /// Builds a greater-than predicate against a bind value.
    pub fn gt(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::Gt, value)
    }

    /// Builds a greater-than-or-equal predicate against a bind value.
    pub fn gte(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::Gte, value)
    }

    /// Builds a less-than predicate against a bind value.
    pub fn lt(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::Lt, value)
    }

    /// Builds a less-than-or-equal predicate against a bind value.
    pub fn lte(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::Lte, value)
    }

    /// Builds an `IS DISTINCT FROM` predicate against a bind value.
    pub fn is_distinct_from(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::IsDistinctFrom, value)
    }

    /// Builds an `IS NOT DISTINCT FROM` predicate against a bind value.
    pub fn is_not_distinct_from(self, value: impl Into<T>) -> BoolExpr {
        self.compare(BoolOp::IsNotDistinctFrom, value)
    }

    /// Builds `field IN (...)`.
    pub fn in_list(self, values: impl IntoIterator<Item = impl Into<T>>) -> BoolExpr {
        self.list_predicate(values, false)
    }

    /// Builds `field NOT IN (...)`.
    pub fn not_in(self, values: impl IntoIterator<Item = impl Into<T>>) -> BoolExpr {
        self.list_predicate(values, true)
    }

    /// Builds `field BETWEEN low AND high`.
    pub fn between(self, low: impl Into<T>, high: impl Into<T>) -> BoolExpr {
        self.between_predicate(low, high, false)
    }

    /// Builds `field NOT BETWEEN low AND high`.
    pub fn not_between(self, low: impl Into<T>, high: impl Into<T>) -> BoolExpr {
        self.between_predicate(low, high, true)
    }

    fn compare(self, op: BoolOp, value: impl Into<T>) -> BoolExpr {
        BoolExpr::Compare {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(value.into())),
        }
    }

    fn list_predicate(
        self,
        values: impl IntoIterator<Item = impl Into<T>>,
        negated: bool,
    ) -> BoolExpr {
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

    fn between_predicate(self, low: impl Into<T>, high: impl Into<T>, negated: bool) -> BoolExpr {
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

impl<T> IntoFieldRef<T> for &Field<T> {
    fn into_field_ref(self) -> FieldRef<T> {
        self.reference()
    }
}

impl<T> IntoFieldRef<T> for FieldRef<T> {
    fn into_field_ref(self) -> FieldRef<T> {
        self
    }
}

impl<T> IntoFieldRef<T> for &FieldRef<T> {
    fn into_field_ref(self) -> FieldRef<T> {
        self.clone()
    }
}

impl<T> From<Field<T>> for ValueExpr {
    fn from(field: Field<T>) -> Self {
        field.expr()
    }
}

impl<T> From<&Field<T>> for ValueExpr {
    fn from(field: &Field<T>) -> Self {
        field.expr()
    }
}

impl<T> From<FieldRef<T>> for ValueExpr {
    fn from(field: FieldRef<T>) -> Self {
        field.expr()
    }
}

impl<T> From<&FieldRef<T>> for ValueExpr {
    fn from(field: &FieldRef<T>) -> Self {
        field.clone().expr()
    }
}
