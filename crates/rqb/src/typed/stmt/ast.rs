use super::*;

#[derive(Clone, Debug)]
pub struct SelectItem {
    pub expr: ValueExpr,
    pub alias: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullsPosition {
    First,
    Last,
}

#[derive(Clone, Debug)]
pub struct OrderItem {
    pub expr: ValueExpr,
    pub direction: OrderDirection,
    pub nulls: Option<NullsPosition>,
}

#[derive(Clone, Debug)]
pub enum GroupByItem {
    Expr(ValueExpr),
    Rollup(Vec<ValueExpr>),
    Cube(Vec<ValueExpr>),
    GroupingSets(Vec<Vec<ValueExpr>>),
}

#[derive(Clone, Debug)]
pub struct FetchClause {
    pub count: ValueExpr,
    pub with_ties: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    Update,
    NoKeyUpdate,
    Share,
    KeyShare,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockWait {
    #[default]
    Wait,
    NoWait,
    SkipLocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowLock {
    pub mode: LockMode,
    pub wait: LockWait,
    pub of: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub field: Meta,
    pub value: ValueExpr,
}

#[derive(Clone, Debug)]
pub enum ConflictTarget {
    Columns {
        fields: Vec<Meta>,
        predicate: Option<Box<BoolExpr>>,
    },
    Constraint(String),
}

#[derive(Clone, Debug)]
pub enum ConflictAction {
    DoNothing,
    DoUpdate {
        assignments: Vec<Assignment>,
        filter: Option<Box<BoolExpr>>,
    },
}

#[derive(Clone, Debug)]
pub struct ConflictClause {
    pub target: ConflictTarget,
    pub action: ConflictAction,
}

/// Builder returned by `Merge::when_matched()`.
#[derive(Clone, Debug)]
pub struct MatchedMergeBuilder {
    pub(super) merge: Merge,
    pub(super) condition: Option<Box<BoolExpr>>,
}

/// Builder returned by `Merge::when_not_matched()`.
#[derive(Clone, Debug)]
pub struct NotMatchedMergeBuilder {
    pub(super) merge: Merge,
    pub(super) condition: Option<Box<BoolExpr>>,
}

/// Builder returned by `Merge::when_not_matched_by_source()`.
#[derive(Clone, Debug)]
pub struct NotMatchedBySourceMergeBuilder {
    pub(super) merge: Merge,
    pub(super) condition: Option<Box<BoolExpr>>,
}

#[derive(Clone, Debug)]
pub struct ColumnConflictBuilder {
    pub(super) insert: Insert,
    pub(super) fields: Vec<Meta>,
    pub(super) predicate: Option<Box<BoolExpr>>,
}

#[derive(Clone, Debug)]
pub struct ConstraintConflictBuilder {
    pub(super) insert: Insert,
    pub(super) constraint: String,
}

pub trait Insertable {
    fn insert_assignments(&self) -> Vec<Assignment>;
}

impl<T> Insertable for &T
where
    T: Insertable + ?Sized,
{
    fn insert_assignments(&self) -> Vec<Assignment> {
        (**self).insert_assignments()
    }
}

pub trait Changeset {
    fn changeset_assignments(&self) -> Vec<Assignment>;
}

impl<T> Changeset for &T
where
    T: Changeset + ?Sized,
{
    fn changeset_assignments(&self) -> Vec<Assignment> {
        (**self).changeset_assignments()
    }
}

/// Column set accepted by `Insert::on_conflict(...)`.
///
/// Use a single field, a tuple of fields, or explicit metadata for dynamic
/// schema code.
pub trait ConflictFields {
    fn conflict_field_count(&self) -> usize;

    fn push_conflict_fields(self, fields: &mut Vec<Meta>);
}

impl<T> ConflictFields for Field<T> {
    fn conflict_field_count(&self) -> usize {
        1
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        push_column(fields, *self.meta);
    }
}

impl ConflictFields for Meta {
    fn conflict_field_count(&self) -> usize {
        1
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        push_column(fields, self);
    }
}

impl ConflictFields for Vec<Meta> {
    fn conflict_field_count(&self) -> usize {
        self.len()
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        for field in self {
            push_column(fields, field);
        }
    }
}

impl<const N: usize> ConflictFields for [Meta; N] {
    fn conflict_field_count(&self) -> usize {
        N
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        for field in self {
            push_column(fields, field);
        }
    }
}

impl<A, B> ConflictFields for (A, B)
where
    A: ConflictFields,
    B: ConflictFields,
{
    fn conflict_field_count(&self) -> usize {
        self.0.conflict_field_count() + self.1.conflict_field_count()
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        let (a, b) = self;
        a.push_conflict_fields(fields);
        b.push_conflict_fields(fields);
    }
}

impl<A, B, C> ConflictFields for (A, B, C)
where
    A: ConflictFields,
    B: ConflictFields,
    C: ConflictFields,
{
    fn conflict_field_count(&self) -> usize {
        self.0.conflict_field_count()
            + self.1.conflict_field_count()
            + self.2.conflict_field_count()
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        let (a, b, c) = self;
        a.push_conflict_fields(fields);
        b.push_conflict_fields(fields);
        c.push_conflict_fields(fields);
    }
}

impl<A, B, C, D> ConflictFields for (A, B, C, D)
where
    A: ConflictFields,
    B: ConflictFields,
    C: ConflictFields,
    D: ConflictFields,
{
    fn conflict_field_count(&self) -> usize {
        self.0.conflict_field_count()
            + self.1.conflict_field_count()
            + self.2.conflict_field_count()
            + self.3.conflict_field_count()
    }

    fn push_conflict_fields(self, fields: &mut Vec<Meta>) {
        let (a, b, c, d) = self;
        a.push_conflict_fields(fields);
        b.push_conflict_fields(fields);
        c.push_conflict_fields(fields);
        d.push_conflict_fields(fields);
    }
}

#[derive(Clone, Debug)]
pub struct Select {
    pub ctes: Vec<Cte>,
    pub source: Source,
    pub joins: Vec<Join>,
    pub distinct: bool,
    pub distinct_on: Vec<ValueExpr>,
    pub projection: Vec<SelectItem>,
    pub filter: Option<BoolExpr>,
    pub group_by: Vec<GroupByItem>,
    pub having: Option<BoolExpr>,
    pub order: Vec<OrderItem>,
    pub limit: Option<Param>,
    pub offset: Option<Param>,
    pub fetch: Option<FetchClause>,
    pub lock: Option<RowLock>,
}

#[derive(Clone, Debug)]
pub struct Insert {
    pub target: Source,
    pub columns: Vec<Meta>,
    pub assignments: Vec<Assignment>,
    pub source: Option<Box<Select>>,
    pub conflict: Option<ConflictClause>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct Update {
    pub target: Source,
    pub assignments: Vec<Assignment>,
    pub from: Vec<Source>,
    pub filter: Option<BoolExpr>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct Delete {
    pub target: Source,
    pub using: Vec<Source>,
    pub filter: Option<BoolExpr>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeWhen {
    Matched,
    NotMatched,
    NotMatchedBySource,
}

#[derive(Clone, Debug)]
pub enum MergeAction {
    DoNothing {
        when: MergeWhen,
        condition: Option<Box<BoolExpr>>,
    },
    Insert {
        condition: Option<Box<BoolExpr>>,
        assignments: Vec<Assignment>,
    },
    Update {
        when: MergeWhen,
        condition: Option<Box<BoolExpr>>,
        assignments: Vec<Assignment>,
    },
    Delete {
        when: MergeWhen,
        condition: Option<Box<BoolExpr>>,
    },
}

#[derive(Clone, Debug)]
pub struct Merge {
    pub ctes: Vec<Cte>,
    pub target: Source,
    pub using: Source,
    pub on: BoolExpr,
    pub actions: Vec<MergeAction>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct RawStmt {
    pub sql: String,
    pub params: Vec<Param>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetOperator {
    Union,
    UnionAll,
    Intersect,
    IntersectAll,
    Except,
    ExceptAll,
}

#[derive(Clone, Debug)]
pub struct SetQuery {
    pub left: Box<Stmt>,
    pub operator: SetOperator,
    pub right: Box<Stmt>,
    pub order: Vec<OrderItem>,
    pub limit: Option<Param>,
    pub offset: Option<Param>,
    pub fetch: Option<FetchClause>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Select(Box<Select>),
    Set(Box<SetQuery>),
    Insert(Box<Insert>),
    Update(Box<Update>),
    Delete(Box<Delete>),
    Merge(Box<Merge>),
    Raw(RawStmt),
}
