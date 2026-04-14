use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::expr::Expr;
use crate::field::FieldRef;
use crate::query::QueryExpr;
use crate::sql_expr::{IntoSqlExpr, SelectItem};
use crate::value::Value;
use crate::write_record::WriteRecord;

use super::{
    ConflictAction, ConflictClause, ConflictTarget, IntoFieldRefs, ReturningMode, WriteAssignment,
};

pub fn insert(dataset: impl Into<Dataset>) -> InsertBuilder {
    InsertBuilder::new(dataset)
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct InsertQuery {
    pub dataset: Dataset,
    pub rows: Vec<Vec<WriteAssignment>>,
    pub source: Option<Box<QueryExpr>>,
    pub returning: ReturningMode,
    pub conflict: Option<ConflictClause>,
}

impl InsertQuery {
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
pub struct InsertBuilder {
    query: InsertQuery,
    errors: Vec<Error>,
}

impl InsertBuilder {
    pub fn new(dataset: impl Into<Dataset>) -> Self {
        Self {
            query: InsertQuery {
                dataset: dataset.into(),
                rows: Vec::new(),
                source: None,
                returning: ReturningMode::none(),
                conflict: None,
            },
            errors: Vec::new(),
        }
    }

    pub fn set(mut self, field: impl Into<FieldRef>, value: impl Into<Value>) -> Self {
        let assignment = WriteAssignment::value(field, value);
        if let Some(row) = self.query.rows.last_mut() {
            row.push(assignment);
        } else {
            self.query.rows.push(vec![assignment]);
        }
        self
    }

    pub fn set_expr(mut self, field: impl Into<FieldRef>, expr: impl IntoSqlExpr) -> Self {
        let assignment = WriteAssignment::expr(field, expr);
        if let Some(row) = self.query.rows.last_mut() {
            row.push(assignment);
        } else {
            self.query.rows.push(vec![assignment]);
        }
        self
    }

    pub fn set_default(mut self, field: impl Into<FieldRef>) -> Self {
        let assignment = WriteAssignment::default(field);
        if let Some(row) = self.query.rows.last_mut() {
            row.push(assignment);
        } else {
            self.query.rows.push(vec![assignment]);
        }
        self
    }

    pub fn value<T>(mut self, record: &T) -> Self
    where
        T: WriteRecord + ?Sized,
    {
        match record.write_fields() {
            Ok(fields) => self.query.rows.push(
                fields
                    .into_iter()
                    .map(|(field, value)| WriteAssignment::value(field, value))
                    .collect(),
            ),
            Err(error) => self.errors.push(error),
        }
        self
    }

    pub fn values<'a, T, I>(mut self, records: I) -> Self
    where
        T: WriteRecord + 'a,
        I: IntoIterator<Item = &'a T>,
    {
        for record in records {
            self = self.value(record);
        }
        self
    }

    pub fn from_select(mut self, select: impl Into<QueryExpr>) -> Self {
        self.query.source = Some(Box::new(select.into()));
        self
    }

    returning_method!();
    apply_method!();

    pub fn on_conflict(self, fields: impl IntoFieldRefs) -> InsertConflictBuilder {
        InsertConflictBuilder {
            builder: self,
            target: ConflictTarget::Columns {
                fields: fields.into_field_refs(),
                predicate: None,
            },
        }
    }

    pub fn on_conflict_constraint(self, constraint: impl Into<String>) -> InsertConflictBuilder {
        InsertConflictBuilder {
            builder: self,
            target: ConflictTarget::Constraint(constraint.into()),
        }
    }

    pub fn conflict_filter(mut self, expr: impl Into<Expr>) -> Self {
        if let Some(ConflictClause {
            action: ConflictAction::DoUpdate { filter, .. },
            ..
        }) = &mut self.query.conflict
        {
            *filter = match filter.take() {
                Some(existing) => Some(existing.and(expr)),
                None => Some(expr.into()),
            };
        } else {
            self.errors.push(Error::InvalidConflictFilter);
        }
        self
    }

    pub fn build(self) -> Result<InsertQuery> {
        if let Some(error) = self.errors.into_iter().next() {
            return Err(error);
        }
        Ok(self.query)
    }
}

#[derive(Clone, Debug)]
#[must_use]
pub struct InsertConflictBuilder {
    builder: InsertBuilder,
    target: ConflictTarget,
}

impl InsertConflictBuilder {
    pub fn index_where(mut self, expr: impl Into<Expr>) -> Self {
        if let ConflictTarget::Columns { predicate, .. } = &mut self.target {
            *predicate = match predicate.take() {
                Some(existing) => Some(Box::new(existing.and(expr))),
                None => Some(Box::new(expr.into())),
            };
        } else {
            self.builder.errors.push(Error::InvalidValue {
                field: "conflict".to_owned(),
                operator: "index_where".to_owned(),
                message: "index_where can only be used with column conflict targets".to_owned(),
            });
        }
        self
    }

    pub fn do_update(self, fields: impl IntoFieldRefs) -> InsertBuilder {
        let assignments = fields
            .into_field_refs()
            .into_iter()
            .map(|field| WriteAssignment::expr(field.clone(), crate::sql_expr::excluded(field)))
            .collect::<Vec<_>>();
        self.do_update_set(assignments)
    }

    pub fn do_update_set<I>(mut self, assignments: I) -> InsertBuilder
    where
        I: IntoIterator<Item = WriteAssignment>,
    {
        self.builder.query.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoUpdate {
                assignments: assignments.into_iter().collect(),
                filter: None,
            },
        });
        self.builder
    }

    pub fn do_nothing(mut self) -> InsertBuilder {
        self.builder.query.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoNothing,
        });
        self.builder
    }
}
