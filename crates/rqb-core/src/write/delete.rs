use crate::dataset::Dataset;
use crate::expr::Expr;
use crate::sql_expr::SelectItem;

use super::{IntoFieldRefs, ReturningMode};

pub fn delete(dataset: impl Into<Dataset>) -> DeleteBuilder {
    DeleteBuilder::new(dataset)
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct DeleteQuery {
    pub dataset: Dataset,
    pub filter: Option<Expr>,
    pub returning: ReturningMode,
}

impl DeleteQuery {
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
pub struct DeleteBuilder {
    query: DeleteQuery,
}

impl DeleteBuilder {
    pub fn new(dataset: impl Into<Dataset>) -> Self {
        Self {
            query: DeleteQuery {
                dataset: dataset.into(),
                filter: None,
                returning: ReturningMode::none(),
            },
        }
    }

    write_filter_methods!();
    returning_method!();
    apply_method!();

    pub fn build(self) -> DeleteQuery {
        self.query
    }
}
