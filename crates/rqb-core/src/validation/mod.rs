mod operators;
mod resolve;
mod scope;
mod select;
mod write;

#[cfg(test)]
mod tests;

use crate::aggregate::{AggregateType, SelectColumn};
use crate::dataset::{Dataset, JoinKind};
use crate::expr::{ColumnOperator, LogicalOp, NullsOrder, Operator, SortDir, SubqueryOperator};
use crate::field::ResolvedField;
use crate::request::RowLock;
use crate::value::Value;
use crate::write::{DeleteQuery, InsertQuery, UpdateQuery};

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedSelect {
    pub dataset: Dataset,
    pub cacheable: bool,
    pub distinct: bool,
    pub ctes: Vec<ValidatedCte>,
    pub joins: Vec<ValidatedJoin>,
    pub selected_fields: Vec<ResolvedField>,
    pub distinct_on: Vec<ResolvedField>,
    pub group_by: Vec<ResolvedField>,
    pub aggregates: Vec<ValidatedAggregate>,
    pub columns: Vec<SelectColumn>,
    pub filter: Option<ValidatedExpr>,
    pub having: Option<ValidatedExpr>,
    pub sort: Vec<ValidatedSort>,
    pub limit: u32,
    pub offset: u64,
    pub limit_explicit: bool,
    pub offset_explicit: bool,
    pub lock: Option<RowLock>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedCte {
    pub name: String,
    pub columns: Vec<String>,
    pub recursive: bool,
    pub body: ValidatedCteBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedCteBody {
    Raw(crate::RawSql),
    Select(Box<ValidatedSelect>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedJoin {
    pub kind: JoinKind,
    pub dataset: Dataset,
    pub on: Option<ValidatedExpr>,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ValidatedExpr {
    Predicate {
        field: ResolvedField,
        operator: Operator,
        value: Value,
    },
    ColumnPredicate {
        left: ResolvedField,
        operator: ColumnOperator,
        right: ResolvedField,
    },
    Subquery {
        field: ResolvedField,
        operator: SubqueryOperator,
        query: Box<ValidatedSelect>,
    },
    Exists {
        query: Box<ValidatedSelect>,
        negated: bool,
    },
    Logical {
        logical: LogicalOp,
        predicates: Vec<ValidatedExpr>,
    },
    Raw(crate::RawSql),
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
        filter: Option<ValidatedExpr>,
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
    pub filter: Option<ValidatedExpr>,
    pub returning: Vec<ResolvedField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedDelete {
    pub query: DeleteQuery,
    pub filter: ValidatedExpr,
    pub returning: Vec<ResolvedField>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedAggregate {
    Count {
        alias: String,
        filter: Option<ValidatedExpr>,
    },
    CountField {
        field: ResolvedField,
        alias: String,
        distinct: bool,
        filter: Option<ValidatedExpr>,
    },
    Sum {
        field: ResolvedField,
        alias: String,
        filter: Option<ValidatedExpr>,
    },
    Avg {
        field: ResolvedField,
        alias: String,
        filter: Option<ValidatedExpr>,
    },
    Min {
        field: ResolvedField,
        alias: String,
        filter: Option<ValidatedExpr>,
    },
    Max {
        field: ResolvedField,
        alias: String,
        filter: Option<ValidatedExpr>,
    },
    JsonAgg {
        alias: String,
        fields: Vec<ResolvedField>,
        order_by: Option<ValidatedSort>,
        filter: Option<ValidatedExpr>,
        default_empty: bool,
    },
    ArrayAgg {
        field: ResolvedField,
        alias: String,
        distinct: bool,
        order_by: Option<ValidatedSort>,
        filter: Option<ValidatedExpr>,
    },
    StringAgg {
        field: ResolvedField,
        separator: String,
        alias: String,
        order_by: Option<ValidatedSort>,
        filter: Option<ValidatedExpr>,
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
