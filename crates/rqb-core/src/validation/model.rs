use crate::aggregate::{AggregateType, SelectColumn};
use crate::dataset::{Dataset, JoinKind};
use crate::expr::{ColumnOperator, LogicalOp, NullsOrder, SortDir, SubqueryOperator};
use crate::field::ResolvedField;
use crate::raw::RawSql;
use crate::request::RowLock;
use crate::types::FieldType;
use crate::value::Value;

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
    pub select_items: Vec<ValidatedSelectItem>,
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
pub struct ValidatedSelectItem {
    pub expr: ValidatedSqlExpr,
    pub alias: String,
    pub ty: FieldType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedSqlExpr {
    Field(ResolvedField),
    Value {
        value: Value,
        ty: FieldType,
    },
    Raw {
        raw: RawSql,
        ty: FieldType,
    },
    Function {
        name: String,
        args: Vec<ValidatedSqlExpr>,
        ty: FieldType,
    },
    Coalesce {
        args: Vec<ValidatedSqlExpr>,
        ty: FieldType,
    },
    Case {
        branches: Vec<ValidatedCaseBranch>,
        otherwise: Box<ValidatedSqlExpr>,
        ty: FieldType,
    },
    Cast {
        expr: Box<ValidatedSqlExpr>,
        ty: FieldType,
    },
}

impl ValidatedSqlExpr {
    pub fn ty(&self) -> FieldType {
        match self {
            Self::Field(field) => field.ty,
            Self::Value { ty, .. }
            | Self::Raw { ty, .. }
            | Self::Function { ty, .. }
            | Self::Coalesce { ty, .. }
            | Self::Case { ty, .. }
            | Self::Cast { ty, .. } => *ty,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedCaseBranch {
    pub condition: ValidatedExpr,
    pub value: ValidatedSqlExpr,
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
    Raw(RawSql),
    Select(Box<ValidatedSelect>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedJoin {
    pub kind: JoinKind,
    pub dataset: Dataset,
    pub on: Option<ValidatedExpr>,
}

#[derive(Clone, Debug, PartialEq)]
// Keep predicates inline to avoid one allocation per expression leaf.
#[allow(clippy::large_enum_variant)]
pub enum ValidatedExpr {
    Predicate(ValidatedPredicate),
    Logical {
        logical: LogicalOp,
        predicates: Vec<ValidatedExpr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValidatedPredicate {
    Raw(RawSql),
    ColumnBinary {
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
    NullCheck {
        field: ResolvedField,
        negated: bool,
    },
    Binary {
        field: ResolvedField,
        op: ValidatedBinaryOperator,
        value: Value,
    },
    NullSafeBinary {
        field: ResolvedField,
        op: ValidatedNullSafeBinaryOperator,
        value: Value,
    },
    In {
        field: ResolvedField,
        values: Vec<Value>,
        negated: bool,
    },
    Between {
        field: ResolvedField,
        lower: Value,
        upper: Value,
        negated: bool,
    },
    Like {
        field: ResolvedField,
        pattern: ValidatedLikePattern,
        value: String,
        negated: bool,
    },
    Regex {
        field: ResolvedField,
        value: String,
        negated: bool,
    },
    TextSearch {
        field: ResolvedField,
        value: String,
    },
    ArraySet {
        field: ResolvedField,
        op: ValidatedArraySetOperator,
        value: Value,
    },
    ArrayMembership {
        field: ResolvedField,
        value: Value,
        negated: bool,
    },
    ArrayState {
        field: ResolvedField,
        empty: bool,
    },
    ArrayElemMatch {
        field: ResolvedField,
        value: Value,
    },
    JsonKey {
        field: ResolvedField,
        key: String,
    },
    JsonKeySet {
        field: ResolvedField,
        keys: Vec<String>,
        all: bool,
    },
    Containment {
        field: ResolvedField,
        op: ValidatedContainmentOperator,
        target: ValidatedContainmentTarget,
        value: Value,
        negated: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatedBinaryOperator {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatedNullSafeBinaryOperator {
    DistinctFrom,
    NotDistinctFrom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatedLikePattern {
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatedArraySetOperator {
    OverlapsAny,
    ContainsAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatedContainmentOperator {
    Contains,
    ContainedBy,
    Overlaps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatedContainmentTarget {
    Range,
    Network,
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
    Raw(RawSql),
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
    pub dataset: Dataset,
    pub target_fields: Vec<ResolvedField>,
    pub rows: Vec<Vec<ValidatedAssignment>>,
    pub from_select: Option<ValidatedSelect>,
    pub returning: Vec<ResolvedField>,
    pub conflict: Option<ValidatedConflictClause>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedUpdate {
    pub dataset: Dataset,
    pub assignments: Vec<ValidatedAssignment>,
    pub filter: Option<ValidatedExpr>,
    pub returning: Vec<ResolvedField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedDelete {
    pub dataset: Dataset,
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
