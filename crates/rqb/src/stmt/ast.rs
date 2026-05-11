use super::*;

/// Projection item in a `SELECT` or `RETURNING` list.
#[derive(Clone, Debug)]
#[must_use]
pub struct SelectItem {
    /// Projected expression.
    pub expr: ValueExpr,
    /// Optional SQL alias.
    pub alias: Option<String>,
}

/// Sort direction for `ORDER BY`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

/// Null placement for `ORDER BY`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullsPosition {
    /// Render `NULLS FIRST`.
    First,
    /// Render `NULLS LAST`.
    Last,
}

/// One `ORDER BY` expression.
#[derive(Clone, Debug)]
#[must_use]
pub struct OrderItem {
    /// Expression to order by.
    pub expr: ValueExpr,
    /// Sort direction.
    pub direction: OrderDirection,
    /// Optional null placement.
    pub nulls: Option<NullsPosition>,
}

/// One `GROUP BY` item.
#[derive(Clone, Debug)]
#[must_use]
pub enum GroupByItem {
    /// Regular grouped expression.
    Expr(ValueExpr),
    /// PostgreSQL `ROLLUP`.
    Rollup(Vec<ValueExpr>),
    /// PostgreSQL `CUBE`.
    Cube(Vec<ValueExpr>),
    /// PostgreSQL `GROUPING SETS`.
    GroupingSets(Vec<Vec<ValueExpr>>),
}

/// SQL `FETCH FIRST` clause.
#[derive(Clone, Debug)]
#[must_use]
pub struct FetchClause {
    /// Row count expression.
    pub count: ValueExpr,
    /// Whether to render `WITH TIES`.
    pub with_ties: bool,
}

/// PostgreSQL row lock mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    /// `FOR UPDATE`.
    Update,
    /// `FOR NO KEY UPDATE`.
    NoKeyUpdate,
    /// `FOR SHARE`.
    Share,
    /// `FOR KEY SHARE`.
    KeyShare,
}

/// PostgreSQL row lock wait behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockWait {
    /// Wait for locked rows.
    #[default]
    Wait,
    /// Render `NOWAIT`.
    NoWait,
    /// Render `SKIP LOCKED`.
    SkipLocked,
}

/// Row lock clause for a select statement.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct RowLock {
    /// Lock mode.
    pub mode: LockMode,
    /// Wait behavior.
    pub wait: LockWait,
    /// Optional relation aliases in `FOR ... OF`.
    pub of: Vec<String>,
}

/// Field assignment used by insert, update, merge, and changesets.
#[derive(Clone, Debug)]
#[must_use]
pub struct Assignment {
    /// Target field metadata.
    pub field: Meta,
    /// Assigned value expression.
    pub value: ValueExpr,
}

/// Converts one or more write assignments into a vector.
///
/// This supports tuple syntax for heterogeneous manual assignments:
/// `insert(users::table()).set_many((users::ID.set(id), users::EMAIL.set(email)))`.
pub trait IntoAssignments {
    /// Converts this value into write assignments.
    fn into_assignments(self) -> Vec<Assignment>;
}

impl IntoAssignments for Assignment {
    fn into_assignments(self) -> Vec<Assignment> {
        vec![self]
    }
}

impl IntoAssignments for &Assignment {
    fn into_assignments(self) -> Vec<Assignment> {
        vec![self.clone()]
    }
}

impl IntoAssignments for Vec<Assignment> {
    fn into_assignments(self) -> Vec<Assignment> {
        self
    }
}

impl IntoAssignments for &[Assignment] {
    fn into_assignments(self) -> Vec<Assignment> {
        self.to_vec()
    }
}

impl<const N: usize> IntoAssignments for [Assignment; N] {
    fn into_assignments(self) -> Vec<Assignment> {
        self.into_iter().collect()
    }
}

macro_rules! impl_assignment_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> IntoAssignments for ($($name,)+)
        where
            $($name: IntoAssignments,)+
        {
            #[allow(non_snake_case)]
            fn into_assignments(self) -> Vec<Assignment> {
                let ($($name,)+) = self;
                let mut assignments = Vec::new();
                $(assignments.extend($name.into_assignments());)+
                assignments
            }
        }
    };
}

impl_assignment_tuple!(A, B);
impl_assignment_tuple!(A, B, C);
impl_assignment_tuple!(A, B, C, D);
impl_assignment_tuple!(A, B, C, D, E);
impl_assignment_tuple!(A, B, C, D, E, F);
impl_assignment_tuple!(A, B, C, D, E, F, G);
impl_assignment_tuple!(A, B, C, D, E, F, G, H);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_assignment_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

/// Target of an `ON CONFLICT` clause.
#[derive(Clone, Debug)]
#[must_use]
pub enum ConflictTarget {
    /// Conflict target by one or more columns.
    Columns {
        /// Target columns.
        fields: Vec<Meta>,
        /// Optional target predicate.
        predicate: Option<Box<BoolExpr>>,
    },
    /// Conflict target by named constraint.
    Constraint(String),
}

/// Action performed for an `ON CONFLICT` clause.
#[derive(Clone, Debug)]
#[must_use]
pub enum ConflictAction {
    /// Render `DO NOTHING`.
    DoNothing,
    /// Render `DO UPDATE SET`.
    DoUpdate {
        /// Update assignments.
        assignments: Vec<Assignment>,
        /// Optional update predicate.
        filter: Option<Box<BoolExpr>>,
    },
}

/// Full `ON CONFLICT` clause.
#[derive(Clone, Debug)]
#[must_use]
pub struct ConflictClause {
    /// Conflict target.
    pub target: ConflictTarget,
    /// Conflict action.
    pub action: ConflictAction,
}

/// Builder returned by `Merge::when_matched()`.
#[derive(Clone, Debug)]
#[must_use]
pub struct MatchedMergeBuilder {
    pub(super) merge: Merge,
    pub(super) condition: Option<Box<BoolExpr>>,
}

/// Builder returned by `Merge::when_not_matched()`.
#[derive(Clone, Debug)]
#[must_use]
pub struct NotMatchedMergeBuilder {
    pub(super) merge: Merge,
    pub(super) condition: Option<Box<BoolExpr>>,
}

/// Builder returned by `Merge::when_not_matched_by_source()`.
#[derive(Clone, Debug)]
#[must_use]
pub struct NotMatchedBySourceMergeBuilder {
    pub(super) merge: Merge,
    pub(super) condition: Option<Box<BoolExpr>>,
}

/// Builder returned by `Insert::on_conflict(...)` for column targets.
#[derive(Clone, Debug)]
#[must_use]
pub struct ColumnConflictBuilder {
    pub(super) insert: Insert,
    pub(super) fields: Vec<Meta>,
    pub(super) predicate: Option<Box<BoolExpr>>,
}

/// Builder returned by `Insert::on_conflict_constraint(...)`.
#[derive(Clone, Debug)]
#[must_use]
pub struct ConstraintConflictBuilder {
    pub(super) insert: Insert,
    pub(super) constraint: String,
}

/// Trait implemented by DTOs that can provide insert assignments.
pub trait Insertable {
    /// Returns assignments to apply to an insert statement.
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

/// Trait implemented by DTOs that can provide update assignments.
pub trait Changeset {
    /// Returns assignments to apply to an update statement.
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
    /// Returns the number of fields this target will push.
    fn conflict_field_count(&self) -> usize;

    /// Pushes conflict target fields into `fields`, preserving order and de-duping.
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

/// Typed select statement.
#[derive(Clone, Debug)]
#[must_use]
pub struct Select {
    /// CTEs attached to this select.
    pub ctes: Vec<Cte>,
    /// Root source.
    pub source: Source,
    /// Joined sources.
    pub joins: Vec<Join>,
    /// Whether to render `DISTINCT`.
    pub distinct: bool,
    /// Expressions for `DISTINCT ON`.
    pub distinct_on: Vec<ValueExpr>,
    /// Projection list. Empty means default root-source projection.
    pub projection: Vec<SelectItem>,
    /// Optional `WHERE` predicate.
    pub filter: Option<BoolExpr>,
    /// Grouping expressions.
    pub group_by: Vec<GroupByItem>,
    /// Optional `HAVING` predicate.
    pub having: Option<BoolExpr>,
    /// Ordering expressions.
    pub order: Vec<OrderItem>,
    /// Optional `LIMIT` parameter.
    pub limit: Option<Param>,
    /// Optional `OFFSET` parameter.
    pub offset: Option<Param>,
    /// Optional SQL `FETCH FIRST` clause.
    pub fetch: Option<FetchClause>,
    /// Optional row lock clause.
    pub lock: Option<RowLock>,
}

/// Typed insert statement.
#[derive(Clone, Debug)]
#[must_use]
pub struct Insert {
    /// Target table or view.
    pub target: Source,
    /// Target columns in insert order.
    pub columns: Vec<Meta>,
    /// Values for insert rows or explicit assignments.
    pub assignments: Vec<Assignment>,
    /// Optional `INSERT ... SELECT` source.
    pub source: Option<Box<Select>>,
    /// Optional conflict handling clause.
    pub conflict: Option<ConflictClause>,
    /// Optional `RETURNING` projection.
    pub returning: Vec<SelectItem>,
}

/// Typed update statement.
#[derive(Clone, Debug)]
#[must_use]
pub struct Update {
    /// CTEs attached to this update.
    pub ctes: Vec<Cte>,
    /// Target table or view.
    pub target: Source,
    /// Assignments for `SET`.
    pub assignments: Vec<Assignment>,
    /// Optional `FROM` sources.
    pub from: Vec<Source>,
    /// Optional `WHERE` predicate.
    pub filter: Option<BoolExpr>,
    /// Optional `RETURNING` projection.
    pub returning: Vec<SelectItem>,
}

/// Typed delete statement.
#[derive(Clone, Debug)]
#[must_use]
pub struct Delete {
    /// CTEs attached to this delete.
    pub ctes: Vec<Cte>,
    /// Target table or view.
    pub target: Source,
    /// Optional `USING` sources.
    pub using: Vec<Source>,
    /// Required `WHERE` predicate.
    pub filter: Option<BoolExpr>,
    /// Optional `RETURNING` projection.
    pub returning: Vec<SelectItem>,
}

/// PostgreSQL `MERGE` match branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeWhen {
    /// `WHEN MATCHED`.
    Matched,
    /// `WHEN NOT MATCHED`.
    NotMatched,
    /// `WHEN NOT MATCHED BY SOURCE`.
    NotMatchedBySource,
}

/// Action inside a PostgreSQL `MERGE` statement.
#[derive(Clone, Debug)]
#[must_use]
pub enum MergeAction {
    /// `DO NOTHING`.
    DoNothing {
        /// Match branch for the action.
        when: MergeWhen,
        /// Optional branch condition.
        condition: Option<Box<BoolExpr>>,
    },
    /// `INSERT`.
    Insert {
        /// Match branch for the action.
        when: MergeWhen,
        /// Optional branch condition.
        condition: Option<Box<BoolExpr>>,
        /// Insert assignments.
        assignments: Vec<Assignment>,
    },
    /// `UPDATE SET`.
    Update {
        /// Match branch for the action.
        when: MergeWhen,
        /// Optional branch condition.
        condition: Option<Box<BoolExpr>>,
        /// Update assignments.
        assignments: Vec<Assignment>,
    },
    /// `DELETE`.
    Delete {
        /// Match branch for the action.
        when: MergeWhen,
        /// Optional branch condition.
        condition: Option<Box<BoolExpr>>,
    },
}

/// Typed PostgreSQL `MERGE` statement.
#[derive(Clone, Debug)]
#[must_use]
pub struct Merge {
    /// CTEs attached to this merge.
    pub ctes: Vec<Cte>,
    /// Merge target.
    pub target: Source,
    /// Merge source.
    pub using: Source,
    /// Join predicate between target and source.
    pub on: BoolExpr,
    /// Ordered merge actions.
    pub actions: Vec<MergeAction>,
    /// Optional `RETURNING` projection.
    pub returning: Vec<SelectItem>,
}

/// Server-owned raw SQL statement.
#[derive(Clone, Debug)]
#[must_use]
pub struct RawStmt {
    /// Raw SQL text using rqb `?` placeholders.
    pub sql: String,
    /// Bind parameters for the raw SQL text.
    pub params: Vec<Param>,
}

/// SQL set query operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetOperator {
    /// `UNION`.
    Union,
    /// `UNION ALL`.
    UnionAll,
    /// `INTERSECT`.
    Intersect,
    /// `INTERSECT ALL`.
    IntersectAll,
    /// `EXCEPT`.
    Except,
    /// `EXCEPT ALL`.
    ExceptAll,
}

/// SQL set query with optional ordering and row limits.
#[derive(Clone, Debug)]
#[must_use]
pub struct SetQuery {
    /// Left query.
    pub left: Box<Stmt>,
    /// Set operator.
    pub operator: SetOperator,
    /// Right query.
    pub right: Box<Stmt>,
    /// Ordering after the set expression.
    pub order: Vec<OrderItem>,
    /// Optional limit.
    pub limit: Option<Param>,
    /// Optional offset.
    pub offset: Option<Param>,
    /// Optional SQL `FETCH FIRST` clause.
    pub fetch: Option<FetchClause>,
}

/// Any top-level query statement rqb can render.
#[derive(Clone, Debug)]
#[must_use]
pub enum Stmt {
    /// Select statement.
    Select(Box<Select>),
    /// Set query.
    Set(Box<SetQuery>),
    /// Insert statement.
    Insert(Box<Insert>),
    /// Update statement.
    Update(Box<Update>),
    /// Delete statement.
    Delete(Box<Delete>),
    /// Merge statement.
    Merge(Box<Merge>),
    /// Raw SQL statement.
    Raw(RawStmt),
}
