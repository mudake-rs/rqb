mod operators;
mod resolve;
mod scope;
mod select;
mod write;

#[cfg(test)]
mod tests;

use crate::aggregate::{AggregateType, SelectColumn};
use crate::expr::{Expr, NullsOrder, SortDir};
use crate::field::ResolvedField;
use crate::request::SelectQuery;
use crate::value::Value;
use crate::write::{DeleteQuery, InsertQuery, UpdateQuery};

pub use resolve::{resolve_field, resolve_query_field, resolve_query_field_with_outer};

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedSelect {
    pub query: SelectQuery,
    pub selected_fields: Vec<ResolvedField>,
    pub distinct_on: Vec<ResolvedField>,
    pub group_by: Vec<ResolvedField>,
    pub aggregates: Vec<ValidatedAggregate>,
    pub columns: Vec<SelectColumn>,
    pub sort: Vec<ValidatedSort>,
    pub limit: u32,
    pub offset: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedSort {
    pub field: ResolvedField,
    pub dir: SortDir,
    pub nulls: Option<NullsOrder>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedAssignment {
    pub field: ResolvedField,
    pub value: ValidatedWriteValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedWriteValue {
    Value(Value),
    Raw(crate::RawSql),
    Column(ResolvedField),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedConflictClause {
    pub target: ValidatedConflictTarget,
    pub action: ValidatedConflictAction,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedConflictTarget {
    Columns(Vec<ResolvedField>),
    Constraint(String),
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ValidatedConflictAction {
    DoNothing,
    DoUpdate {
        fields: Vec<ResolvedField>,
        filter: Option<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedInsert {
    pub query: InsertQuery,
    pub rows: Vec<Vec<ValidatedAssignment>>,
    pub from_select: Option<ValidatedSelect>,
    pub from_select_targets: Vec<ResolvedField>,
    pub returning: Vec<ResolvedField>,
    pub conflict: Option<ValidatedConflictClause>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedUpdate {
    pub query: UpdateQuery,
    pub assignments: Vec<ValidatedAssignment>,
    pub returning: Vec<ResolvedField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedDelete {
    pub query: DeleteQuery,
    pub returning: Vec<ResolvedField>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedAggregate {
    Count {
        alias: String,
        filter: Option<Expr>,
    },
    CountField {
        field: ResolvedField,
        alias: String,
        distinct: bool,
        filter: Option<Expr>,
    },
    Sum {
        field: ResolvedField,
        alias: String,
        filter: Option<Expr>,
    },
    Avg {
        field: ResolvedField,
        alias: String,
        filter: Option<Expr>,
    },
    Min {
        field: ResolvedField,
        alias: String,
        filter: Option<Expr>,
    },
    Max {
        field: ResolvedField,
        alias: String,
        filter: Option<Expr>,
    },
    JsonAgg {
        alias: String,
        fields: Vec<ResolvedField>,
        order_by: Option<ValidatedSort>,
        filter: Option<Expr>,
        default_empty: bool,
    },
    ArrayAgg {
        field: ResolvedField,
        alias: String,
        distinct: bool,
        order_by: Option<ValidatedSort>,
        filter: Option<Expr>,
    },
    StringAgg {
        field: ResolvedField,
        separator: String,
        alias: String,
        order_by: Option<ValidatedSort>,
        filter: Option<Expr>,
    },
}

impl ValidatedAggregate {
    pub fn alias(&self) -> &str {
        match self {
            Self::Count { alias, .. }
            | Self::CountField { alias, .. }
            | Self::Sum { alias, .. }
            | Self::Avg { alias, .. }
            | Self::Min { alias, .. }
            | Self::Max { alias, .. }
            | Self::JsonAgg { alias, .. }
            | Self::ArrayAgg { alias, .. }
            | Self::StringAgg { alias, .. } => alias,
        }
    }

    pub fn aggregate_type(&self) -> AggregateType {
        match self {
            Self::Count { .. } | Self::CountField { .. } => AggregateType::Count,
            Self::Sum { .. } => AggregateType::Sum,
            Self::Avg { .. } => AggregateType::Avg,
            Self::Min { field, .. } => AggregateType::Min(field.ty),
            Self::Max { field, .. } => AggregateType::Max(field.ty),
            Self::JsonAgg { .. } | Self::ArrayAgg { .. } => AggregateType::Json,
            Self::StringAgg { .. } => AggregateType::String,
        }
    }
}
