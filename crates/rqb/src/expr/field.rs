use std::marker::PhantomData;

use crate::{BindValue, Meta, Param, SelectItem};

use super::{BoolExpr, ValueExpr};

/// Typed database field generated from schema metadata.
///
/// Generated schema exposes `Field<T>` constants for unqualified columns. Use
/// `field.at("alias")` or the generated `alias("u").field()` handle when the
/// same relation appears in joins or subqueries.
#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub struct Field<T> {
    /// Static field metadata.
    pub meta: &'static Meta,
    _ty: PhantomData<T>,
}

/// Qualified typed database field, usually created through an alias handle.
///
/// `FieldRef<T>` keeps the same metadata as the base field but renders with a
/// table/source qualifier.
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
#[doc(hidden)]
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

    #[inline]
    pub(super) fn reference(self) -> FieldRef<T> {
        FieldRef {
            meta: self.meta,
            qualifier: None,
            _ty: PhantomData,
        }
    }

    /// Returns this field as a value expression.
    #[inline]
    pub fn expr(self) -> ValueExpr {
        self.reference().expr()
    }

    /// Returns this field qualified as `old.field` for PostgreSQL 18 DML `RETURNING`.
    #[inline]
    pub fn old_value(self) -> FieldRef<T> {
        self.at("old")
    }

    /// Returns this field qualified as `new.field` for PostgreSQL 18 DML `RETURNING`.
    #[inline]
    pub fn new_value(self) -> FieldRef<T> {
        self.at("new")
    }

    /// Builds a custom value operator expression for this field.
    ///
    /// This is an escape hatch for server-owned PostgreSQL operators not yet
    /// modeled by rqb. Bound values inside the right-hand expression remain
    /// parameters, but PostgreSQL is the final authority on operator validity.
    pub fn op(self, op: &'static str, right: impl Into<ValueExpr>) -> ValueExpr {
        self.reference().op(op, right)
    }

    /// Builds a custom boolean infix predicate for this field.
    ///
    /// This is an escape hatch for server-owned PostgreSQL operators not yet
    /// modeled by rqb. Prefer typed helpers when one exists.
    pub fn predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().predicate(op, right)
    }

    /// Builds a negated custom boolean infix predicate for this field.
    pub fn not_predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().not_predicate(op, right)
    }

    /// Returns this field as an aliased projection item.
    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        self.expr().alias(alias)
    }

    /// Returns `EXCLUDED.field` for `ON CONFLICT DO UPDATE`.
    #[inline]
    pub fn excluded(self) -> ValueExpr {
        ValueExpr::Excluded(*self.meta)
    }

    /// Creates an assignment from an expression.
    pub fn set_expr(self, value: impl Into<ValueExpr>) -> crate::Assignment {
        crate::Assignment {
            field: *self.meta,
            value: crate::AssignmentValue::Expr(value.into()),
        }
    }

    /// Creates an assignment that writes SQL `NULL`.
    #[inline]
    pub fn set_null(self) -> crate::Assignment {
        self.set_expr(ValueExpr::Null)
    }

    /// Creates an assignment that writes SQL `DEFAULT`.
    #[inline]
    pub fn set_default(self) -> crate::Assignment {
        crate::Assignment {
            field: *self.meta,
            value: crate::AssignmentValue::Default,
        }
    }

    /// Creates an assignment from the same field exposed by another source alias.
    ///
    /// This keeps sync-style updates and `MERGE` actions from repeating both
    /// sides of `target.field = incoming.field`.
    pub fn set_from(self, alias: impl Into<String>) -> crate::Assignment {
        crate::Assignment {
            field: *self.meta,
            value: crate::AssignmentValue::Expr(self.at(alias).expr()),
        }
    }

    /// Creates an assignment from `EXCLUDED.field`.
    #[inline]
    pub fn set_excluded(self) -> crate::Assignment {
        crate::Assignment {
            field: *self.meta,
            value: crate::AssignmentValue::Expr(self.excluded()),
        }
    }

    /// Builds `field IS NULL`.
    #[inline]
    pub fn is_null(self) -> BoolExpr {
        self.reference().is_null()
    }

    /// Builds `field IS NOT NULL`.
    #[inline]
    pub fn is_not_null(self) -> BoolExpr {
        self.reference().is_not_null()
    }

    /// Builds `field IN (subquery)`.
    pub fn in_subquery(self, query: impl Into<crate::Stmt>) -> BoolExpr {
        self.reference().in_subquery(query)
    }

    /// Builds `field NOT IN (subquery)`.
    pub fn not_in_subquery(self, query: impl Into<crate::Stmt>) -> BoolExpr {
        self.reference().not_in_subquery(query)
    }

    /// Builds an equality predicate against another field of the same Rust type.
    pub fn eq_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().eq_field(right)
    }

    /// Builds an inequality predicate against another field of the same Rust type.
    pub fn ne_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().ne_field(right)
    }

    /// Builds a greater-than predicate against another field of the same Rust type.
    pub fn gt_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().gt_field(right)
    }

    /// Builds a greater-than-or-equal predicate against another field of the same Rust type.
    pub fn gte_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().gte_field(right)
    }

    /// Builds a less-than predicate against another field of the same Rust type.
    pub fn lt_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().lt_field(right)
    }

    /// Builds a less-than-or-equal predicate against another field of the same Rust type.
    pub fn lte_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().lte_field(right)
    }

    /// Builds `IS DISTINCT FROM` against another field of the same Rust type.
    pub fn is_distinct_from_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().is_distinct_from_field(right)
    }

    /// Builds `IS NOT DISTINCT FROM` against another field of the same Rust type.
    pub fn is_not_distinct_from_field<R>(self, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        self.reference().is_not_distinct_from_field(right)
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
        self.reference().eq(value)
    }

    /// Builds an inequality predicate against a bind value.
    pub fn ne(self, value: impl Into<T>) -> BoolExpr {
        self.reference().ne(value)
    }

    /// Builds a greater-than predicate against a bind value.
    pub fn gt(self, value: impl Into<T>) -> BoolExpr {
        self.reference().gt(value)
    }

    /// Builds a greater-than-or-equal predicate against a bind value.
    pub fn gte(self, value: impl Into<T>) -> BoolExpr {
        self.reference().gte(value)
    }

    /// Builds a less-than predicate against a bind value.
    pub fn lt(self, value: impl Into<T>) -> BoolExpr {
        self.reference().lt(value)
    }

    /// Builds a less-than-or-equal predicate against a bind value.
    pub fn lte(self, value: impl Into<T>) -> BoolExpr {
        self.reference().lte(value)
    }

    /// Builds an `IS DISTINCT FROM` predicate against a bind value.
    pub fn is_distinct_from(self, value: impl Into<T>) -> BoolExpr {
        self.reference().is_distinct_from(value)
    }

    /// Builds an `IS NOT DISTINCT FROM` predicate against a bind value.
    pub fn is_not_distinct_from(self, value: impl Into<T>) -> BoolExpr {
        self.reference().is_not_distinct_from(value)
    }

    /// Builds `field IN (...)`.
    pub fn in_list(self, values: impl IntoIterator<Item = impl Into<T>>) -> BoolExpr {
        self.reference().in_list(values)
    }

    /// Builds `field NOT IN (...)`.
    pub fn not_in(self, values: impl IntoIterator<Item = impl Into<T>>) -> BoolExpr {
        self.reference().not_in(values)
    }

    /// Builds `field BETWEEN low AND high`.
    pub fn between(self, low: impl Into<T>, high: impl Into<T>) -> BoolExpr {
        self.reference().between(low, high)
    }

    /// Builds `field NOT BETWEEN low AND high`.
    pub fn not_between(self, low: impl Into<T>, high: impl Into<T>) -> BoolExpr {
        self.reference().not_between(low, high)
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
