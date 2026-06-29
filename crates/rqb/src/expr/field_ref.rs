use crate::{BindValue, Param};

use super::{BoolExpr, BoolOp, FieldRef, IntoFieldRef, ValueExpr};

impl<T> FieldRef<T> {
    /// Returns this qualified field as a value expression.
    #[inline]
    pub fn expr(self) -> ValueExpr {
        ValueExpr::field(*self.meta, self.qualifier)
    }

    /// Builds a custom value operator expression for this field reference.
    pub fn op(self, op: &'static str, right: impl Into<ValueExpr>) -> ValueExpr {
        self.expr().op(op, right)
    }

    /// Builds a custom boolean infix predicate for this field reference.
    pub fn predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        self.expr().predicate(op, right)
    }

    /// Builds a negated custom boolean infix predicate for this field reference.
    pub fn not_predicate(self, op: &'static str, right: impl Into<ValueExpr>) -> BoolExpr {
        self.expr().not_predicate(op, right)
    }

    /// Builds `field IS NULL`.
    #[inline]
    pub fn is_null(self) -> BoolExpr {
        BoolExpr::is_null_expr(self.expr(), false)
    }

    /// Builds `field IS NOT NULL`.
    #[inline]
    pub fn is_not_null(self) -> BoolExpr {
        BoolExpr::is_null_expr(self.expr(), true)
    }

    /// Builds `field IN (subquery)`.
    pub fn in_subquery(self, query: impl Into<crate::Stmt>) -> BoolExpr {
        BoolExpr::in_subquery(self.expr(), Box::new(query.into()), false)
    }

    /// Builds `field NOT IN (subquery)`.
    pub fn not_in_subquery(self, query: impl Into<crate::Stmt>) -> BoolExpr {
        BoolExpr::in_subquery(self.expr(), Box::new(query.into()), true)
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

    fn compare_field<R>(self, op: BoolOp, right: R) -> BoolExpr
    where
        R: IntoFieldRef<T>,
    {
        BoolExpr::compare(self.expr(), op, right.into_field_ref().expr())
    }
}

impl<T: BindValue> FieldRef<T> {
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
        BoolExpr::compare(
            self.expr(),
            op,
            ValueExpr::Param(Param::typed(value.into())),
        )
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
        BoolExpr::in_list(self.expr(), values, negated)
    }

    fn between_predicate(self, low: impl Into<T>, high: impl Into<T>, negated: bool) -> BoolExpr {
        BoolExpr::between(
            self.expr(),
            ValueExpr::Param(Param::typed(low.into())),
            ValueExpr::Param(Param::typed(high.into())),
            negated,
        )
    }
}
