use std::marker::PhantomData;

use sqlx::{Encode, Postgres, Type, postgres::PgHasArrayType};

use crate::typed::{Meta, OrderItem, Param, SelectItem, raw};
use crate::{Error, Result};

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

    fn reference(self) -> FieldRef<T> {
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

impl<T> FieldRef<T> {
    pub fn expr(self) -> ValueExpr {
        ValueExpr::Field {
            meta: *self.meta,
            qualifier: self.qualifier,
        }
    }

    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        self.expr().alias(alias)
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

impl Field<String> {
    pub fn like(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().like(pattern)
    }

    pub fn not_like(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_like(pattern)
    }

    pub fn ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().ilike(pattern)
    }

    pub fn not_ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_ilike(pattern)
    }

    pub fn contains(self, value: impl Into<String>) -> BoolExpr {
        self.reference().contains(value)
    }

    pub fn not_contains(self, value: impl Into<String>) -> BoolExpr {
        self.reference().not_contains(value)
    }

    pub fn starts_with(self, value: impl Into<String>) -> BoolExpr {
        self.reference().starts_with(value)
    }

    pub fn not_starts_with(self, value: impl Into<String>) -> BoolExpr {
        self.reference().not_starts_with(value)
    }

    pub fn ends_with(self, value: impl Into<String>) -> BoolExpr {
        self.reference().ends_with(value)
    }

    pub fn not_ends_with(self, value: impl Into<String>) -> BoolExpr {
        self.reference().not_ends_with(value)
    }

    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().regex(pattern)
    }

    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_regex(pattern)
    }

    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().iregex(pattern)
    }

    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.reference().not_iregex(pattern)
    }
}

impl FieldRef<String> {
    pub fn like(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, false, false)
    }

    pub fn not_like(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, false, true)
    }

    pub fn ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, true, false)
    }

    pub fn not_ilike(self, pattern: impl Into<String>) -> BoolExpr {
        self.like_predicate(pattern, true, true)
    }

    pub fn contains(self, value: impl Into<String>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", false)
    }

    pub fn not_contains(self, value: impl Into<String>) -> BoolExpr {
        self.affix_predicate(value, "%", "%", true)
    }

    pub fn starts_with(self, value: impl Into<String>) -> BoolExpr {
        self.affix_predicate(value, "", "%", false)
    }

    pub fn not_starts_with(self, value: impl Into<String>) -> BoolExpr {
        self.affix_predicate(value, "", "%", true)
    }

    pub fn ends_with(self, value: impl Into<String>) -> BoolExpr {
        self.affix_predicate(value, "%", "", false)
    }

    pub fn not_ends_with(self, value: impl Into<String>) -> BoolExpr {
        self.affix_predicate(value, "%", "", true)
    }

    pub fn regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, false)
    }

    pub fn not_regex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, false, true)
    }

    pub fn iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, false)
    }

    pub fn not_iregex(self, pattern: impl Into<String>) -> BoolExpr {
        self.regex_predicate(pattern, true, true)
    }

    fn like_predicate(
        self,
        pattern: impl Into<String>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Like {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(pattern.into())),
            case_insensitive,
            negated,
            escape: false,
        }
    }

    fn affix_predicate(
        self,
        value: impl Into<String>,
        prefix: &'static str,
        suffix: &'static str,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Like {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(format!(
                "{prefix}{}{suffix}",
                escape_like(&value.into())
            ))),
            case_insensitive: true,
            negated,
            escape: true,
        }
    }

    fn regex_predicate(
        self,
        pattern: impl Into<String>,
        case_insensitive: bool,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Regex {
            expr: self.expr(),
            pattern: ValueExpr::Param(Param::typed(pattern.into())),
            case_insensitive,
            negated,
        }
    }
}

impl<T> Field<Vec<T>>
where
    T: Clone
        + Send
        + Sync
        + 'static
        + for<'q> Encode<'q, Postgres>
        + Type<Postgres>
        + PgHasArrayType,
{
    pub fn contains_any(self, values: Vec<T>) -> BoolExpr {
        self.reference().contains_any(values)
    }

    pub fn contains_all(self, values: Vec<T>) -> BoolExpr {
        self.reference().contains_all(values)
    }

    pub fn contained_by(self, values: Vec<T>) -> BoolExpr {
        self.reference().contained_by(values)
    }

    pub fn has(self, value: T) -> BoolExpr {
        self.reference().has(value)
    }

    pub fn not_has(self, value: T) -> BoolExpr {
        self.reference().not_has(value)
    }

    pub fn is_empty(self) -> BoolExpr {
        self.reference().is_empty()
    }

    pub fn is_not_empty(self) -> BoolExpr {
        self.reference().is_not_empty()
    }
}

impl<T> FieldRef<Vec<T>>
where
    T: Clone
        + Send
        + Sync
        + 'static
        + for<'q> Encode<'q, Postgres>
        + Type<Postgres>
        + PgHasArrayType,
{
    pub fn contains_any(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("&&", values, false)
    }

    pub fn contains_all(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("@>", values, false)
    }

    pub fn contained_by(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("<@", values, false)
    }

    pub fn has(self, value: T) -> BoolExpr {
        self.any_predicate(value, false)
    }

    pub fn not_has(self, value: T) -> BoolExpr {
        self.any_predicate(value, true)
    }

    pub fn is_empty(self) -> BoolExpr {
        BoolExpr::ArrayIsEmpty {
            expr: self.expr(),
            negated: false,
        }
    }

    pub fn is_not_empty(self) -> BoolExpr {
        BoolExpr::ArrayIsEmpty {
            expr: self.expr(),
            negated: true,
        }
    }

    fn array_infix(self, op: &'static str, values: Vec<T>, negated: bool) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(values)),
            negated,
        }
    }

    fn any_predicate(self, value: T, negated: bool) -> BoolExpr {
        BoolExpr::Any {
            value: ValueExpr::Param(Param::typed(value)),
            array: self.expr(),
            negated,
        }
    }
}

impl Field<serde_json::Value> {
    pub fn contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_contains(value)
    }

    pub fn contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_contained_by(value)
    }

    pub fn key_exists(self, key: impl Into<String>) -> BoolExpr {
        self.reference().key_exists(key)
    }

    pub fn keys_exist_any(self, keys: Vec<String>) -> BoolExpr {
        self.reference().keys_exist_any(keys)
    }

    pub fn keys_exist_all(self, keys: Vec<String>) -> BoolExpr {
        self.reference().keys_exist_all(keys)
    }

    pub fn json_contains(self, value: serde_json::Value) -> BoolExpr {
        self.reference().json_contains(value)
    }

    pub fn json_contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.reference().json_contained_by(value)
    }
}

impl<T> Field<sqlx::postgres::types::PgRange<T>>
where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    sqlx::postgres::types::PgRange<T>:
        Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
{
    pub fn contains(self, value: T) -> BoolExpr {
        self.range_contains(value)
    }

    pub fn range_contains(self, value: T) -> BoolExpr {
        self.reference().range_contains(value)
    }

    pub fn contains_range(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().contains_range(value)
    }

    pub fn contained_by(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().contained_by(value)
    }

    pub fn overlaps(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().overlaps(value)
    }
}

impl<T> FieldRef<sqlx::postgres::types::PgRange<T>>
where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    sqlx::postgres::types::PgRange<T>:
        Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
{
    pub fn contains(self, value: T) -> BoolExpr {
        self.range_contains(value)
    }

    pub fn range_contains(self, value: T) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op: "@>",
            right: ValueExpr::Param(Param::typed(value)),
            negated: false,
        }
    }

    pub fn contains_range(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("@>", value)
    }

    pub fn contained_by(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("<@", value)
    }

    pub fn overlaps(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("&&", value)
    }

    fn range_infix(self, op: &'static str, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(value)),
            negated: false,
        }
    }
}

impl FieldRef<serde_json::Value> {
    pub fn contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_contains(value)
    }

    pub fn contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_contained_by(value)
    }

    pub fn key_exists(self, key: impl Into<String>) -> BoolExpr {
        self.json_infix("?", Param::typed(key.into()))
    }

    pub fn keys_exist_any(self, keys: Vec<String>) -> BoolExpr {
        self.json_infix("?|", Param::typed(keys))
    }

    pub fn keys_exist_all(self, keys: Vec<String>) -> BoolExpr {
        self.json_infix("?&", Param::typed(keys))
    }

    pub fn json_contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_infix("@>", Param::typed(value))
    }

    pub fn json_contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_infix("<@", Param::typed(value))
    }

    fn json_infix(self, op: &'static str, param: Param) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(param),
            negated: false,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    IsDistinctFrom,
    IsNotDistinctFrom,
}

impl BoolOp {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::IsDistinctFrom => "IS DISTINCT FROM",
            Self::IsNotDistinctFrom => "IS NOT DISTINCT FROM",
        }
    }

    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::IsDistinctFrom => "is_distinct_from",
            Self::IsNotDistinctFrom => "is_not_distinct_from",
        }
    }

    pub const fn requires_ordering(self) -> bool {
        matches!(self, Self::Gt | Self::Gte | Self::Lt | Self::Lte)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOp {
    Add,
    Sub,
    Mul,
    Div,
    Custom(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFunction {
    RowNumber,
    Rank,
    DenseRank,
    Lag,
    Lead,
}

#[derive(Clone, Debug, Default)]
pub struct WindowSpec {
    pub partition_by: Vec<ValueExpr>,
    pub order_by: Vec<OrderItem>,
}

#[derive(Clone, Debug)]
pub struct WindowFunctionBuilder {
    function: WindowFunction,
    args: Vec<ValueExpr>,
}

#[derive(Clone, Debug)]
pub struct OffsetWindowFunctionBuilder {
    function: WindowFunction,
    value: ValueExpr,
    offset: Option<ValueExpr>,
    default: Option<ValueExpr>,
}

#[derive(Clone, Debug)]
pub enum BoolExpr {
    Constant(bool),
    Compare {
        left: ValueExpr,
        op: BoolOp,
        right: ValueExpr,
    },
    IsNull {
        expr: ValueExpr,
        negated: bool,
    },
    InList {
        expr: ValueExpr,
        values: Vec<ValueExpr>,
        negated: bool,
    },
    InSubquery {
        expr: ValueExpr,
        query: Box<crate::typed::Stmt>,
        negated: bool,
    },
    Between {
        expr: ValueExpr,
        low: ValueExpr,
        high: ValueExpr,
        negated: bool,
    },
    Like {
        expr: ValueExpr,
        pattern: ValueExpr,
        case_insensitive: bool,
        negated: bool,
        escape: bool,
    },
    Regex {
        expr: ValueExpr,
        pattern: ValueExpr,
        case_insensitive: bool,
        negated: bool,
    },
    Infix {
        left: ValueExpr,
        op: &'static str,
        right: ValueExpr,
        negated: bool,
    },
    Any {
        value: ValueExpr,
        array: ValueExpr,
        negated: bool,
    },
    ArrayIsEmpty {
        expr: ValueExpr,
        negated: bool,
    },
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Not(Box<BoolExpr>),
    Exists(Box<crate::typed::Stmt>),
    Raw {
        sql: String,
        params: Vec<Param>,
    },
}

#[derive(Clone, Debug)]
pub enum ValueExpr {
    Field {
        meta: Meta,
        qualifier: Option<String>,
    },
    Excluded(Meta),
    Param(Param),
    Function {
        name: &'static str,
        args: Vec<ValueExpr>,
    },
    Aggregate {
        name: &'static str,
        args: Vec<ValueExpr>,
        distinct: bool,
        order_by: Vec<OrderItem>,
        filter: Option<Box<BoolExpr>>,
    },
    Case {
        branches: Vec<(BoolExpr, ValueExpr)>,
        else_: Option<Box<ValueExpr>>,
    },
    Cast {
        expr: Box<ValueExpr>,
        pg: &'static str,
    },
    Binary {
        left: Box<ValueExpr>,
        op: ValueOp,
        right: Box<ValueExpr>,
    },
    Window {
        function: WindowFunction,
        args: Vec<ValueExpr>,
        spec: WindowSpec,
    },
    Raw {
        sql: String,
        params: Vec<Param>,
    },
    Subquery(Box<crate::typed::Stmt>),
}

impl BoolExpr {
    pub fn and(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::And(exprs.into_iter().collect())
    }

    pub fn or(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::Or(exprs.into_iter().collect())
    }

    pub fn negate(expr: BoolExpr) -> Self {
        Self::Not(Box::new(expr))
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Constant(_) => Ok(()),
            Self::Compare { left, op, right } => {
                validate_compare(left, *op)?;
                left.validate()?;
                right.validate()
            }
            Self::IsNull { expr, .. } => expr.validate(),
            Self::InList { expr, values, .. } => {
                validate_equality_expr(expr, "in")?;
                expr.validate()?;
                for value in values {
                    value.validate()?;
                }
                Ok(())
            }
            Self::InSubquery { expr, query, .. } => {
                validate_equality_expr(expr, "in_subquery")?;
                expr.validate()?;
                query.validate()
            }
            Self::Between {
                expr, low, high, ..
            } => {
                validate_ordered_expr(expr, "between")?;
                expr.validate()?;
                low.validate()?;
                high.validate()
            }
            Self::Like { expr, pattern, .. } => {
                validate_like_expr(expr)?;
                expr.validate()?;
                pattern.validate()
            }
            Self::Regex { expr, pattern, .. } => {
                validate_like_expr(expr)?;
                expr.validate()?;
                pattern.validate()
            }
            Self::Infix {
                left, op, right, ..
            } => {
                validate_infix_expr(left, op)?;
                left.validate()?;
                right.validate()
            }
            Self::Any { value, array, .. } => {
                validate_array_expr(array, "any")?;
                value.validate()?;
                array.validate()
            }
            Self::ArrayIsEmpty { expr, .. } => {
                validate_array_expr(expr, "array_empty")?;
                expr.validate()
            }
            Self::And(exprs) | Self::Or(exprs) => {
                if exprs.is_empty() {
                    return Err(Error::EmptyTypedLogical {
                        logical: match self {
                            Self::And(_) => "and",
                            Self::Or(_) => "or",
                            _ => unreachable!(),
                        }
                        .to_owned(),
                    });
                }
                for expr in exprs {
                    expr.validate()?;
                }
                Ok(())
            }
            Self::Not(expr) => expr.validate(),
            Self::Exists(stmt) => stmt.validate(),
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
        }
    }

    pub fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Constant(_) => {}
            Self::Compare { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::IsNull { expr, .. } => expr.collect_params(params),
            Self::InList { expr, values, .. } => {
                expr.collect_params(params);
                for value in values {
                    value.collect_params(params);
                }
            }
            Self::InSubquery { expr, query, .. } => {
                expr.collect_params(params);
                query.collect_params(params);
            }
            Self::Between {
                expr, low, high, ..
            } => {
                expr.collect_params(params);
                low.collect_params(params);
                high.collect_params(params);
            }
            Self::Like { expr, pattern, .. } => {
                expr.collect_params(params);
                pattern.collect_params(params);
            }
            Self::Regex { expr, pattern, .. } => {
                expr.collect_params(params);
                pattern.collect_params(params);
            }
            Self::Infix { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::Any { value, array, .. } => {
                value.collect_params(params);
                array.collect_params(params);
            }
            Self::ArrayIsEmpty { expr, .. } => expr.collect_params(params),
            Self::And(exprs) | Self::Or(exprs) => {
                for expr in exprs {
                    expr.collect_params(params);
                }
            }
            Self::Not(expr) => expr.collect_params(params),
            Self::Exists(stmt) => stmt.collect_params(params),
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
        }
    }
}

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

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Aggregate {
                filter,
                args,
                order_by,
                ..
            } => {
                for arg in args {
                    arg.validate()?;
                }
                for item in order_by {
                    item.validate()?;
                }
                if let Some(filter) = filter {
                    filter.validate()?;
                }
                Ok(())
            }
            Self::Function { args, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                Ok(())
            }
            Self::Case { branches, else_ } => {
                for (when, then) in branches {
                    when.validate()?;
                    then.validate()?;
                }
                if let Some(else_) = else_ {
                    else_.validate()?;
                }
                Ok(())
            }
            Self::Cast { expr, .. } => expr.validate(),
            Self::Binary { left, right, .. } => {
                left.validate()?;
                right.validate()
            }
            Self::Window { args, spec, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                for expr in &spec.partition_by {
                    expr.validate()?;
                }
                for item in &spec.order_by {
                    item.validate()?;
                }
                Ok(())
            }
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
            Self::Subquery(stmt) => stmt.validate(),
            Self::Field { .. } | Self::Excluded(_) | Self::Param(_) => Ok(()),
        }
    }

    pub(crate) fn field_meta(&self) -> Option<&Meta> {
        match self {
            Self::Field { meta, .. } => Some(meta),
            _ => None,
        }
    }

    pub fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Param(param) => params.push(param.clone()),
            Self::Function { args, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
            }
            Self::Aggregate {
                args,
                order_by,
                filter,
                ..
            } => {
                for arg in args {
                    arg.collect_params(params);
                }
                for item in order_by {
                    item.collect_params(params);
                }
                if let Some(filter) = filter {
                    filter.collect_params(params);
                }
            }
            Self::Case { branches, else_ } => {
                for (when, then) in branches {
                    when.collect_params(params);
                    then.collect_params(params);
                }
                if let Some(else_) = else_ {
                    else_.collect_params(params);
                }
            }
            Self::Cast { expr, .. } => expr.collect_params(params),
            Self::Binary { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::Window { args, spec, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
                for expr in &spec.partition_by {
                    expr.collect_params(params);
                }
                for item in &spec.order_by {
                    item.collect_params(params);
                }
            }
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
            Self::Subquery(stmt) => stmt.collect_params(params),
            Self::Field { .. } | Self::Excluded(_) => {}
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

impl From<Param> for ValueExpr {
    fn from(param: Param) -> Self {
        Self::Param(param)
    }
}

fn validate_compare(left: &ValueExpr, op: BoolOp) -> Result<()> {
    let Some(meta) = left.field_meta() else {
        return Ok(());
    };
    let supported = if op.requires_ordering() {
        meta.ops.ordering
    } else {
        meta.ops.equality
    };
    if supported {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: op.as_name().to_owned(),
    })
}

fn validate_equality_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.equality {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: operator.to_owned(),
    })
}

fn validate_ordered_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.ops.ordering {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: operator.to_owned(),
    })
}

fn validate_like_expr(expr: &ValueExpr) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if matches!(meta.pg, "text" | "varchar" | "bpchar" | "citext") {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: "like".to_owned(),
    })
}

fn validate_infix_expr(expr: &ValueExpr, op: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    let supported = match op {
        "?" | "?|" | "?&" => matches!(meta.pg, "jsonb"),
        "@>" | "<@" | "&&" => {
            meta.pg.ends_with("[]")
                || matches!(
                    meta.pg,
                    "jsonb"
                        | "int4range"
                        | "int8range"
                        | "numrange"
                        | "daterange"
                        | "tsrange"
                        | "tstzrange"
                        | "inet"
                        | "cidr"
                )
        }
        _ => true,
    };
    if supported {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: op.to_owned(),
    })
}

fn validate_array_expr(expr: &ValueExpr, operator: &'static str) -> Result<()> {
    let Some(meta) = expr.field_meta() else {
        return Ok(());
    };
    if meta.pg.ends_with("[]") {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: operator.to_owned(),
    })
}

pub(crate) fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::typed::{BoolOp, Field, JsonKind, Meta, OpSet, Param, Params, ValueExpr};

    #[test]
    fn field_t_erases_to_bool_expr_with_sqlx_param() {
        static ID_META: Meta = Meta::new("id", "id", "uuid")
            .ops(OpSet::equality())
            .json(JsonKind::Uuid);
        const ID: Field<Uuid> = Field::new(&ID_META);

        let expr = ID.eq(Uuid::nil());
        expr.validate().unwrap();

        let mut raw_params = Vec::new();
        expr.collect_params(&mut raw_params);
        let params = Params::from_vec(raw_params);

        assert_eq!(params.len(), 1);
        assert!(params.debug_names()[0].ends_with("uuid::Uuid"));
    }

    #[test]
    fn field_is_copy_without_requiring_t_to_be_copy() {
        static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
        const EMAIL: Field<String> = Field::new(&EMAIL_META);

        let field = EMAIL;
        let _first = field.expr();
        let _second = field.expr();
    }

    #[test]
    fn operator_validation_uses_meta_not_rust_type_traits() {
        static PAYLOAD_META: Meta = Meta::new("payload", "payload", "jsonb")
            .json(JsonKind::Jsonb)
            .ops(OpSet::equality());
        const PAYLOAD: Field<serde_json::Value> = Field::new(&PAYLOAD_META);

        let err = PAYLOAD
            .gt(serde_json::json!({ "n": 1 }))
            .validate()
            .unwrap_err();

        assert!(matches!(
            err,
            crate::Error::InvalidTypedOperator { field, operator }
                if field == "payload" && operator == "gt"
        ));
    }

    #[test]
    fn value_expr_is_separate_from_bool_expr() {
        static EMAIL_META: Meta = Meta::new("email", "email", "text")
            .ops(OpSet::ordered())
            .json(JsonKind::Text);
        const EMAIL: Field<String> = Field::new(&EMAIL_META);

        let lower = ValueExpr::Function {
            name: "lower",
            args: vec![EMAIL.expr()],
        };
        let filter = crate::typed::BoolExpr::Compare {
            left: lower,
            op: BoolOp::Eq,
            right: ValueExpr::Param(Param::typed("egor@example.com".to_owned())),
        };

        filter.validate().unwrap();
    }

    #[test]
    fn meta_defaults_to_no_typed_operators() {
        static SCORE_META: Meta = Meta::new("score", "score", "int4");
        const SCORE: Field<i32> = Field::new(&SCORE_META);

        let err = SCORE.eq(10).validate().unwrap_err();

        assert!(matches!(
            err,
            crate::Error::InvalidTypedOperator { field, operator }
                if field == "score" && operator == "eq"
        ));
    }

    #[test]
    fn raw_predicate_validates_bind_count() {
        let err = crate::typed::BoolExpr::Raw {
            sql: "score > ? and active = ?".to_owned(),
            params: vec![Param::typed(10_i32)],
        }
        .validate()
        .unwrap_err();

        assert!(matches!(
            err,
            crate::Error::RawBindMismatch {
                placeholders: 2,
                binds: 1
            }
        ));
    }
}
