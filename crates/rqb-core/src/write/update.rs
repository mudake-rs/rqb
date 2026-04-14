use serde::Serialize;

use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::expr::Expr;
use crate::field::FieldRef;
use crate::raw::RawSql;
use crate::serde_bridge::fields_from_serializable;
use crate::sql_expr::{IntoSqlExpr, SelectItem};
use crate::value::Value;

use super::{IntoFieldRefs, ReturningMode, WriteAssignment};

pub fn update(dataset: impl Into<Dataset>) -> UpdateBuilder {
    UpdateBuilder::new(dataset)
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct UpdateQuery {
    pub dataset: Dataset,
    pub assignments: Vec<WriteAssignment>,
    pub filter: Option<Expr>,
    pub returning: ReturningMode,
}

impl UpdateQuery {
    pub fn returning(mut self, fields: impl IntoFieldRefs) -> Self {
        self.returning.set_fields(fields);
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning.set_all();
        self
    }

    pub fn returning_expr(mut self, item: SelectItem) -> Self {
        self.returning.push_expr(item);
        self
    }

    pub fn returning_all_if_empty(mut self) -> Self {
        if self.returning.is_none() {
            self.returning.set_all();
        }
        self
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct UpdateBuilder {
    query: UpdateQuery,
    errors: Vec<Error>,
}

impl UpdateBuilder {
    pub fn new(dataset: impl Into<Dataset>) -> Self {
        Self {
            query: UpdateQuery {
                dataset: dataset.into(),
                assignments: Vec::new(),
                filter: None,
                returning: ReturningMode::none(),
            },
            errors: Vec::new(),
        }
    }

    pub fn set(mut self, field: impl Into<FieldRef>, value: impl Into<Value>) -> Self {
        self.query
            .assignments
            .push(WriteAssignment::value(field, value));
        self
    }

    pub fn set_expr(mut self, field: impl Into<FieldRef>, expr: impl IntoSqlExpr) -> Self {
        self.query
            .assignments
            .push(WriteAssignment::expr(field, expr));
        self
    }

    pub fn set_default(mut self, field: impl Into<FieldRef>) -> Self {
        self.query.assignments.push(WriteAssignment::default(field));
        self
    }

    pub fn set_null(self, field: impl Into<FieldRef>) -> Self {
        self.set(field, Value::Null)
    }

    pub fn set_from<T>(mut self, record: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        match fields_from_serializable(&self.query.dataset, record) {
            Ok(fields) => self.query.assignments.extend(
                fields
                    .into_iter()
                    .map(|(field, value)| WriteAssignment::value(field, value)),
            ),
            Err(error) => self.errors.push(error),
        }
        self
    }

    pub fn set_raw(mut self, field: impl Into<FieldRef>, raw: RawSql) -> Self {
        self.query
            .assignments
            .push(WriteAssignment::raw(field, raw));
        self
    }

    pub fn set_col(mut self, field: impl Into<FieldRef>, source: impl Into<FieldRef>) -> Self {
        self.query
            .assignments
            .push(WriteAssignment::column(field, source));
        self
    }

    write_filter_methods!();
    returning_method!();
    apply_method!();

    pub fn build(self) -> Result<UpdateQuery> {
        if let Some(error) = self.errors.into_iter().next() {
            return Err(error);
        }
        Ok(self.query)
    }
}
