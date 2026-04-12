use crate::field::{Field, FieldRef};
use crate::raw::RawSql;
use crate::value::Value;

pub trait IntoFieldRefs {
    fn into_field_refs(self) -> Vec<FieldRef>;
}

impl IntoFieldRefs for Field {
    fn into_field_refs(self) -> Vec<FieldRef> {
        vec![self.into()]
    }
}

impl IntoFieldRefs for FieldRef {
    fn into_field_refs(self) -> Vec<FieldRef> {
        vec![self]
    }
}

impl IntoFieldRefs for &str {
    fn into_field_refs(self) -> Vec<FieldRef> {
        vec![self.into()]
    }
}

impl IntoFieldRefs for String {
    fn into_field_refs(self) -> Vec<FieldRef> {
        vec![self.into()]
    }
}

impl<F, const N: usize> IntoFieldRefs for [F; N]
where
    F: Into<FieldRef>,
{
    fn into_field_refs(self) -> Vec<FieldRef> {
        self.into_iter().map(Into::into).collect()
    }
}

impl<F> IntoFieldRefs for Vec<F>
where
    F: Into<FieldRef>,
{
    fn into_field_refs(self) -> Vec<FieldRef> {
        self.into_iter().map(Into::into).collect()
    }
}

impl<F> IntoFieldRefs for &[F]
where
    F: Clone + Into<FieldRef>,
{
    fn into_field_refs(self) -> Vec<FieldRef> {
        self.iter().cloned().map(Into::into).collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum ReturningMode {
    None,
    All,
    Fields(Vec<FieldRef>),
}

impl ReturningMode {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

macro_rules! write_filter_methods {
    () => {
        pub fn filter(self, expr: impl Into<Expr>) -> Self {
            self.and_where(expr)
        }

        pub fn replace_filter(mut self, expr: impl Into<Expr>) -> Self {
            self.query.filter = Some(expr.into());
            self
        }

        pub fn and_where(mut self, expr: impl Into<Expr>) -> Self {
            self.query.filter = match self.query.filter.take() {
                Some(existing) => Some(existing.and(expr)),
                None => Some(expr.into()),
            };
            self
        }

        pub fn or_where(mut self, expr: impl Into<Expr>) -> Self {
            self.query.filter = match self.query.filter.take() {
                Some(existing) => Some(existing.or(expr)),
                None => Some(expr.into()),
            };
            self
        }

        pub fn filter_if(self, condition: bool, expr: impl Into<Expr>) -> Self {
            if condition {
                self.and_where(expr)
            } else {
                self
            }
        }

        pub fn filter_option<V, F>(self, value: Option<V>, f: F) -> Self
        where
            F: FnOnce(V) -> Expr,
        {
            match value {
                Some(value) => self.and_where(f(value)),
                None => self,
            }
        }
    };
}

macro_rules! returning_method {
    () => {
        pub fn returning(mut self, fields: impl IntoFieldRefs) -> Self {
            self.query.returning = ReturningMode::Fields(fields.into_field_refs());
            self
        }

        pub fn returning_all(mut self) -> Self {
            self.query.returning = ReturningMode::All;
            self
        }

        pub fn returning_all_if_empty(mut self) -> Self {
            if self.query.returning.is_none() {
                self.query.returning = ReturningMode::All;
            }
            self
        }
    };
}

macro_rules! apply_method {
    () => {
        pub fn apply<F>(self, f: F) -> Self
        where
            F: FnOnce(Self) -> Self,
        {
            f(self)
        }
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct WriteAssignment {
    pub field: FieldRef,
    pub value: WriteValue,
}

impl WriteAssignment {
    pub fn value(field: impl Into<FieldRef>, value: impl Into<Value>) -> Self {
        Self {
            field: field.into(),
            value: WriteValue::Value(value.into()),
        }
    }

    pub fn raw(field: impl Into<FieldRef>, raw: RawSql) -> Self {
        Self {
            field: field.into(),
            value: WriteValue::Raw(raw),
        }
    }

    pub fn column(field: impl Into<FieldRef>, source: impl Into<FieldRef>) -> Self {
        Self {
            field: field.into(),
            value: WriteValue::Column(source.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WriteValue {
    Value(Value),
    Raw(RawSql),
    Column(FieldRef),
}

mod conflict;
mod delete;
mod insert;
mod update;

pub use conflict::{ConflictAction, ConflictClause, ConflictTarget};
pub use delete::{DeleteBuilder, DeleteQuery, delete};
pub use insert::{InsertBuilder, InsertConflictBuilder, InsertQuery, insert};
pub use update::{UpdateBuilder, UpdateQuery, update};
