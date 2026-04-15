use crate::typed::{
    BoolExpr, Cte, Field, FieldRef, Join, JoinKind, Meta, Param, Params, Source, ValueExpr,
    raw as raw_sql, subquery,
};
use crate::{Error, Result};

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

#[derive(Clone, Debug)]
pub struct OrderItem {
    pub expr: ValueExpr,
    pub direction: OrderDirection,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowLock {
    pub mode: LockMode,
    pub wait: LockWait,
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
    #[doc(hidden)]
    Invalid {
        message: &'static str,
    },
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

#[derive(Clone, Debug)]
pub struct InsertConflictBuilder {
    insert: Insert,
    target: ConflictTarget,
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

#[derive(Clone, Debug)]
pub struct Select {
    pub ctes: Vec<Cte>,
    pub source: Source,
    pub joins: Vec<Join>,
    pub distinct: bool,
    pub distinct_on: Vec<ValueExpr>,
    pub projection: Vec<SelectItem>,
    pub filter: Option<BoolExpr>,
    pub group_by: Vec<ValueExpr>,
    pub having: Option<BoolExpr>,
    pub order: Vec<OrderItem>,
    pub limit: Option<Param>,
    pub offset: Option<Param>,
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
    pub filter: Option<BoolExpr>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug)]
pub struct Delete {
    pub target: Source,
    pub filter: Option<BoolExpr>,
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
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Select(Box<Select>),
    Set(Box<SetQuery>),
    Insert(Box<Insert>),
    Update(Box<Update>),
    Delete(Box<Delete>),
    Raw(RawStmt),
}

impl SelectItem {
    pub fn new(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            alias: None,
        }
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

impl<T> From<Field<T>> for SelectItem {
    fn from(field: Field<T>) -> Self {
        select_item_for_field(field)
    }
}

impl<T> From<FieldRef<T>> for SelectItem {
    fn from(field: FieldRef<T>) -> Self {
        select_item_for_ref(field)
    }
}

impl Assignment {
    pub fn new<T>(field: Field<T>, value: impl Into<ValueExpr>) -> Self {
        Self {
            field: *field.meta,
            value: value.into(),
        }
    }
}

impl OrderDirection {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl LockMode {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Update => "FOR UPDATE",
            Self::NoKeyUpdate => "FOR NO KEY UPDATE",
            Self::Share => "FOR SHARE",
            Self::KeyShare => "FOR KEY SHARE",
        }
    }
}

impl LockWait {
    pub const fn as_sql(self) -> Option<&'static str> {
        match self {
            Self::Wait => None,
            Self::NoWait => Some("NOWAIT"),
            Self::SkipLocked => Some("SKIP LOCKED"),
        }
    }
}

impl RowLock {
    pub const fn new(mode: LockMode) -> Self {
        Self {
            mode,
            wait: LockWait::Wait,
        }
    }

    pub const fn nowait(mut self) -> Self {
        self.wait = LockWait::NoWait;
        self
    }

    pub const fn skip_locked(mut self) -> Self {
        self.wait = LockWait::SkipLocked;
        self
    }
}

impl Default for RowLock {
    fn default() -> Self {
        Self::new(LockMode::Update)
    }
}

impl OrderItem {
    pub fn asc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Asc,
        }
    }

    pub fn desc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Desc,
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.expr.validate()?;
        if let Some(meta) = self.expr.field_meta()
            && !meta.ops.ordering
        {
            return Err(Error::InvalidTypedSort {
                field: meta.api.to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        self.expr.collect_params(params);
    }
}

pub fn select(source: Source) -> Select {
    Select::from(source)
}

pub fn insert(target: Source) -> Insert {
    Insert::into(target)
}

pub fn update(target: Source) -> Update {
    Update::table(target)
}

pub fn delete_from(target: Source) -> Delete {
    Delete::from(target)
}

pub fn raw(sql: impl Into<String>) -> RawStmt {
    RawStmt::new(sql)
}

pub fn union(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::Union, left, right)
}

pub fn union_all(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::UnionAll, left, right)
}

pub fn intersect(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::Intersect, left, right)
}

pub fn intersect_all(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::IntersectAll, left, right)
}

pub fn except(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::Except, left, right)
}

pub fn except_all(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::ExceptAll, left, right)
}

impl Stmt {
    pub fn raw(sql: impl Into<String>) -> Self {
        Self::Raw(RawStmt::new(sql))
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Select(select) => select.validate(),
            Self::Set(set) => set.validate(),
            Self::Insert(insert) => insert.validate(),
            Self::Update(update) => update.validate(),
            Self::Delete(delete) => delete.validate(),
            Self::Raw(raw_stmt) => raw_stmt.validate(),
        }
    }

    pub fn params(&self) -> Params {
        let mut params = Vec::new();
        self.collect_params(&mut params);
        Params::from_vec(params)
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Select(select) => select.collect_params(params),
            Self::Set(set) => set.collect_params(params),
            Self::Insert(insert) => insert.collect_params(params),
            Self::Update(update) => update.collect_params(params),
            Self::Delete(delete) => delete.collect_params(params),
            Self::Raw(raw_stmt) => params.extend(raw_stmt.params.iter().cloned()),
        }
    }
}

impl From<Select> for Stmt {
    fn from(select: Select) -> Self {
        Self::Select(Box::new(select))
    }
}

impl From<SetQuery> for Stmt {
    fn from(set: SetQuery) -> Self {
        Self::Set(Box::new(set))
    }
}

impl SetOperator {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Union => "UNION",
            Self::UnionAll => "UNION ALL",
            Self::Intersect => "INTERSECT",
            Self::IntersectAll => "INTERSECT ALL",
            Self::Except => "EXCEPT",
            Self::ExceptAll => "EXCEPT ALL",
        }
    }
}

impl SetQuery {
    pub fn new(operator: SetOperator, left: impl Into<Stmt>, right: impl Into<Stmt>) -> Self {
        Self {
            left: Box::new(left.into()),
            operator,
            right: Box::new(right.into()),
            order: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn union(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::Union, self, right)
    }

    pub fn union_all(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::UnionAll, self, right)
    }

    pub fn intersect(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::Intersect, self, right)
    }

    pub fn intersect_all(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::IntersectAll, self, right)
    }

    pub fn except(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::Except, self, right)
    }

    pub fn except_all(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::ExceptAll, self, right)
    }

    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order.push(item);
        self
    }

    pub fn order_asc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc(expr));
        self
    }

    pub fn order_desc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc(expr));
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(Param::typed(i64::from(limit)));
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(Param::typed(i64::from(offset)));
        self
    }

    pub fn into_source(self, alias: impl Into<String>, fields: impl Into<Vec<Meta>>) -> Source {
        subquery(self, alias, fields)
    }

    pub fn validate(&self) -> Result<()> {
        self.left.validate()?;
        self.right.validate()?;
        for item in &self.order {
            item.validate()?;
        }
        Ok(())
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        self.left.collect_params(params);
        self.right.collect_params(params);
        for item in &self.order {
            item.collect_params(params);
        }
        if let Some(limit) = &self.limit {
            params.push(limit.clone());
        }
        if let Some(offset) = &self.offset {
            params.push(offset.clone());
        }
    }
}

impl Select {
    pub fn union(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::Union, self, right)
    }

    pub fn union_all(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::UnionAll, self, right)
    }

    pub fn intersect(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::Intersect, self, right)
    }

    pub fn intersect_all(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::IntersectAll, self, right)
    }

    pub fn except(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::Except, self, right)
    }

    pub fn except_all(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::ExceptAll, self, right)
    }
}

impl Select {
    pub fn from(source: Source) -> Self {
        Self {
            ctes: Vec::new(),
            source,
            joins: Vec::new(),
            distinct: false,
            distinct_on: Vec::new(),
            projection: Vec::new(),
            filter: None,
            group_by: Vec::new(),
            having: None,
            order: Vec::new(),
            limit: None,
            offset: None,
            lock: None,
        }
    }

    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    pub fn into_source(self, alias: impl Into<String>, fields: impl Into<Vec<Meta>>) -> Source {
        subquery(self, alias, fields)
    }

    pub fn column(mut self, field: impl Into<SelectItem>) -> Self {
        self.projection.push(field.into());
        self
    }

    pub fn expr(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.projection.push(SelectItem {
            expr: expr.into(),
            alias: None,
        });
        self
    }

    pub fn item(mut self, item: SelectItem) -> Self {
        self.projection.push(item);
        self
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
        self
    }

    pub fn replace_filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn filter_if(self, condition: bool, filter: BoolExpr) -> Self {
        if condition { self.filter(filter) } else { self }
    }

    pub fn filter_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> BoolExpr) -> Self {
        match value {
            Some(value) => self.filter(f(value)),
            None => self,
        }
    }

    pub fn apply(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    pub fn distinct_on(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.distinct_on.push(expr.into());
        self
    }

    pub fn join(mut self, source: Source, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Inner, source, on));
        self
    }

    pub fn left_join(mut self, source: Source, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Left, source, on));
        self
    }

    pub fn right_join(mut self, source: Source, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Right, source, on));
        self
    }

    pub fn full_join(mut self, source: Source, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Full, source, on));
        self
    }

    pub fn join_lateral(mut self, source: Source, on: BoolExpr) -> Self {
        self.joins.push(Join::lateral(JoinKind::Inner, source, on));
        self
    }

    pub fn left_join_lateral(mut self, source: Source, on: BoolExpr) -> Self {
        self.joins.push(Join::lateral(JoinKind::Left, source, on));
        self
    }

    pub fn cross_join(mut self, source: Source) -> Self {
        self.joins.push(Join::cross(source));
        self
    }

    pub fn cross_join_lateral(mut self, source: Source) -> Self {
        self.joins.push(Join::cross_lateral(source));
        self
    }

    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order.push(item);
        self
    }

    pub fn group_by(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.group_by.push(expr.into());
        self
    }

    pub fn having(mut self, having: BoolExpr) -> Self {
        self.having = Some(match self.having {
            Some(existing) => BoolExpr::And(vec![existing, having]),
            None => having,
        });
        self
    }

    pub fn order_asc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc(expr));
        self
    }

    pub fn order_desc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc(expr));
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(Param::typed(i64::from(limit)));
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(Param::typed(i64::from(offset)));
        self
    }

    pub fn lock(mut self, mode: LockMode) -> Self {
        self.lock = Some(RowLock::new(mode));
        self
    }

    pub fn for_update(self) -> Self {
        self.lock(LockMode::Update)
    }

    pub fn for_no_key_update(self) -> Self {
        self.lock(LockMode::NoKeyUpdate)
    }

    pub fn for_share(self) -> Self {
        self.lock(LockMode::Share)
    }

    pub fn for_key_share(self) -> Self {
        self.lock(LockMode::KeyShare)
    }

    pub fn nowait(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().nowait());
        self
    }

    pub fn skip_locked(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().skip_locked());
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_cte_names(&self.ctes)?;
        for cte in &self.ctes {
            cte.validate()?;
        }
        self.source.validate()?;
        for join in &self.joins {
            join.validate()?;
        }
        for expr in &self.distinct_on {
            expr.validate()?;
        }
        for item in &self.projection {
            item.expr.validate()?;
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        for expr in &self.group_by {
            expr.validate()?;
        }
        if let Some(having) = &self.having {
            having.validate()?;
        }
        for item in &self.order {
            item.validate()?;
        }
        Ok(())
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        for cte in &self.ctes {
            cte.collect_params(params);
        }
        for expr in &self.distinct_on {
            expr.collect_params(params);
        }
        for item in &self.projection {
            item.expr.collect_params(params);
        }
        self.source.collect_from_params(params);
        for join in &self.joins {
            join.collect_params(params);
        }
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        for expr in &self.group_by {
            expr.collect_params(params);
        }
        if let Some(having) = &self.having {
            having.collect_params(params);
        }
        for item in &self.order {
            item.collect_params(params);
        }
        if let Some(limit) = &self.limit {
            params.push(limit.clone());
        }
        if let Some(offset) = &self.offset {
            params.push(offset.clone());
        }
    }
}

impl Insert {
    pub fn into(target: Source) -> Self {
        Self {
            target,
            columns: Vec::new(),
            assignments: Vec::new(),
            source: None,
            conflict: None,
            returning: Vec::new(),
        }
    }

    /// Adds one column assignment. If the same database column was assigned
    /// earlier, this assignment replaces the earlier value.
    ///
    /// This makes it safe to layer server-owned values around a DTO mapping:
    /// call `values(&dto)` for request-owned fields and use `set(...)` for
    /// generated IDs, tenant IDs, status defaults, or explicit overrides.
    pub fn set(mut self, assignment: Assignment) -> Self {
        push_column(&mut self.columns, assignment.field);
        push_assignment(&mut self.assignments, assignment);
        self
    }

    pub fn values(mut self, values: impl Insertable) -> Self {
        extend_insert_assignments(
            &mut self.columns,
            &mut self.assignments,
            values.insert_assignments(),
        );
        self
    }

    pub fn column<T>(mut self, field: Field<T>) -> Self {
        push_column(&mut self.columns, *field.meta);
        self
    }

    pub fn from_select(mut self, select: Select) -> Self {
        self.source = Some(Box::new(select));
        self
    }

    pub fn on_conflict<T>(self, field: Field<T>) -> InsertConflictBuilder {
        InsertConflictBuilder {
            insert: self,
            target: ConflictTarget::Columns {
                fields: vec![*field.meta],
                predicate: None,
            },
        }
    }

    pub fn on_conflict_constraint(self, constraint: impl Into<String>) -> InsertConflictBuilder {
        InsertConflictBuilder {
            insert: self,
            target: ConflictTarget::Constraint(constraint.into()),
        }
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_table_target("insert", &self.target)?;
        match (&self.source, self.assignments.is_empty()) {
            (Some(source), true) => {
                validate_nonempty_columns("insert-select", &self.columns)?;
                validate_insert_select_columns(&self.columns, source)?;
                source.validate()?;
            }
            (Some(_), false) => {
                return Err(Error::InvalidInsertShape {
                    message: "insert-select cannot also contain VALUES assignments",
                });
            }
            (None, true) => validate_nonempty_assignments("insert", &self.assignments)?,
            (None, false) => {
                for assignment in &self.assignments {
                    assignment.value.validate()?;
                }
            }
        }
        if let Some(conflict) = &self.conflict {
            conflict.validate()?;
        }
        validate_returning(&self.returning)
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        if let Some(source) = &self.source {
            source.collect_params(params);
        } else {
            for assignment in &self.assignments {
                assignment.value.collect_params(params);
            }
        }
        if let Some(conflict) = &self.conflict {
            conflict.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl InsertConflictBuilder {
    /// Adds another column to an `ON CONFLICT (columns...)` target.
    ///
    /// This is valid only after [`Insert::on_conflict`]. If it is called after
    /// [`Insert::on_conflict_constraint`], the insert fails validation before rendering.
    pub fn and<T>(mut self, field: Field<T>) -> Self {
        match &mut self.target {
            ConflictTarget::Columns { fields, .. } => push_column(fields, *field.meta),
            ConflictTarget::Constraint(_) => {
                invalidate_conflict_target(
                    &mut self.target,
                    "and requires on_conflict(column), not on_conflict_constraint",
                );
            }
            ConflictTarget::Invalid { .. } => {}
        }
        self
    }

    /// Adds an index predicate to an `ON CONFLICT (columns...)` target.
    ///
    /// Repeated calls are AND-combined. This is valid only after
    /// [`Insert::on_conflict`]; using it after [`Insert::on_conflict_constraint`]
    /// fails validation before rendering.
    pub fn target_where(mut self, predicate: BoolExpr) -> Self {
        match &mut self.target {
            ConflictTarget::Columns {
                predicate: current, ..
            } => {
                *current = Some(Box::new(match current.take() {
                    Some(existing) => BoolExpr::And(vec![*existing, predicate]),
                    None => predicate,
                }));
            }
            ConflictTarget::Constraint(_) => {
                invalidate_conflict_target(
                    &mut self.target,
                    "target_where requires on_conflict(column), not on_conflict_constraint",
                );
            }
            ConflictTarget::Invalid { .. } => {}
        }
        self
    }

    pub fn do_nothing(mut self) -> Insert {
        self.insert.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoNothing,
        });
        self.insert
    }

    pub fn do_update_set<I>(mut self, assignments: I) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        self.insert.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoUpdate {
                assignments: assignments.into_iter().collect(),
                filter: None,
            },
        });
        self.insert
    }

    pub fn do_update_set_where<I>(mut self, assignments: I, filter: BoolExpr) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        self.insert.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoUpdate {
                assignments: assignments.into_iter().collect(),
                filter: Some(Box::new(filter)),
            },
        });
        self.insert
    }
}

fn invalidate_conflict_target(target: &mut ConflictTarget, message: &'static str) {
    if !matches!(target, ConflictTarget::Invalid { .. }) {
        *target = ConflictTarget::Invalid { message };
    }
}

impl ConflictClause {
    fn validate(&self) -> Result<()> {
        match &self.target {
            ConflictTarget::Columns { fields, predicate } => {
                validate_nonempty_columns("conflict", fields)?;
                if let Some(predicate) = predicate {
                    predicate.validate()?;
                }
            }
            ConflictTarget::Constraint(constraint) if constraint.is_empty() => {
                return Err(Error::InvalidInsertShape {
                    message: "conflict constraint name cannot be empty",
                });
            }
            ConflictTarget::Constraint(_) => {}
            ConflictTarget::Invalid { message } => {
                return Err(Error::InvalidInsertShape { message });
            }
        }
        if let ConflictAction::DoUpdate {
            assignments,
            filter,
        } = &self.action
        {
            validate_nonempty_assignments("conflict update", assignments)?;
            for assignment in assignments {
                assignment.value.validate()?;
            }
            if let Some(filter) = filter {
                filter.validate()?;
            }
        }
        Ok(())
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        if let ConflictTarget::Columns {
            predicate: Some(predicate),
            ..
        } = &self.target
        {
            predicate.collect_params(params);
        }
        if let ConflictAction::DoUpdate {
            assignments,
            filter,
        } = &self.action
        {
            for assignment in assignments {
                assignment.value.collect_params(params);
            }
            if let Some(filter) = filter {
                filter.collect_params(params);
            }
        }
    }
}

impl Update {
    pub fn table(target: Source) -> Self {
        Self {
            target,
            assignments: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn set(mut self, assignment: Assignment) -> Self {
        push_assignment(&mut self.assignments, assignment);
        self
    }

    pub fn changes(mut self, changes: impl Changeset) -> Self {
        extend_assignments(&mut self.assignments, changes.changeset_assignments());
        self
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
        self
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_table_target("update", &self.target)?;
        validate_nonempty_assignments("update", &self.assignments)?;
        for assignment in &self.assignments {
            assignment.value.validate()?;
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        validate_returning(&self.returning)
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        for assignment in &self.assignments {
            assignment.value.collect_params(params);
        }
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl Delete {
    pub fn from(target: Source) -> Self {
        Self {
            target,
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
        self
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_table_target("delete", &self.target)?;
        let Some(filter) = &self.filter else {
            return Err(Error::TypedDeleteWithoutFilter);
        };
        filter.validate()?;
        validate_returning(&self.returning)
    }

    fn collect_params(&self, params: &mut Vec<Param>) {
        if let Some(filter) = &self.filter {
            filter.collect_params(params);
        }
        collect_returning_params(&self.returning, params);
    }
}

impl RawStmt {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        raw_sql::validate_bind_count(&self.sql, self.params.len())
    }

    pub fn bind<T>(mut self, value: T) -> Self
    where
        T: Clone
            + Send
            + Sync
            + 'static
            + for<'q> sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>,
    {
        self.params.push(Param::typed(value));
        self
    }
}

fn validate_table_target(statement: &'static str, target: &Source) -> Result<()> {
    if target.is_table() {
        return Ok(());
    }
    Err(Error::InvalidTypedWriteTarget {
        statement,
        source_kind: target.kind(),
    })
}

fn validate_nonempty_assignments(
    statement: &'static str,
    assignments: &[Assignment],
) -> Result<()> {
    if assignments.is_empty() {
        return Err(Error::EmptyTypedAssignments { statement });
    }
    Ok(())
}

fn extend_insert_assignments(
    columns: &mut Vec<Meta>,
    assignments: &mut Vec<Assignment>,
    next: Vec<Assignment>,
) {
    for assignment in next {
        push_column(columns, assignment.field);
        push_assignment(assignments, assignment);
    }
}

fn extend_assignments(assignments: &mut Vec<Assignment>, next: Vec<Assignment>) {
    for assignment in next {
        push_assignment(assignments, assignment);
    }
}

fn push_column(columns: &mut Vec<Meta>, field: Meta) {
    if !columns.iter().any(|existing| existing.db == field.db) {
        columns.push(field);
    }
}

fn push_assignment(assignments: &mut Vec<Assignment>, assignment: Assignment) {
    assignments.retain(|existing| existing.field.db != assignment.field.db);
    assignments.push(assignment);
}

fn validate_nonempty_columns(statement: &'static str, columns: &[Meta]) -> Result<()> {
    if columns.is_empty() {
        return Err(Error::EmptyTypedColumns { statement });
    }
    Ok(())
}

fn validate_cte_names(ctes: &[Cte]) -> Result<()> {
    let mut seen = Vec::<&str>::new();
    for cte in ctes {
        if seen.contains(&cte.name.as_str()) {
            return Err(Error::InvalidCteShape {
                name: cte.name.clone(),
                message: "duplicate CTE name",
            });
        }
        seen.push(cte.name.as_str());
    }
    Ok(())
}

fn validate_insert_select_columns(columns: &[Meta], source: &Select) -> Result<()> {
    if let Some(count) = source.projection_count()
        && count != columns.len()
    {
        return Err(Error::InvalidInsertShape {
            message: "insert-select column count must match SELECT projection count",
        });
    }
    Ok(())
}

impl Select {
    fn projection_count(&self) -> Option<usize> {
        if !self.projection.is_empty() {
            return Some(self.projection.len());
        }
        let mut count = 0usize;
        self.source.for_each_field(|_| count += 1);
        (count > 0).then_some(count)
    }
}

fn validate_returning(returning: &[SelectItem]) -> Result<()> {
    for item in returning {
        item.expr.validate()?;
    }
    Ok(())
}

fn collect_returning_params(returning: &[SelectItem], params: &mut Vec<Param>) {
    for item in returning {
        item.expr.collect_params(params);
    }
}

fn select_item_for_field<T>(field: Field<T>) -> SelectItem {
    let alias = field_alias(field.meta);
    SelectItem {
        expr: field.expr(),
        alias,
    }
}

fn select_item_for_ref<T>(field: FieldRef<T>) -> SelectItem {
    let alias = field_ref_alias(&field);
    SelectItem {
        expr: field.expr(),
        alias,
    }
}

fn select_item_for_meta(meta: Meta) -> SelectItem {
    let alias = field_alias(&meta);
    SelectItem {
        expr: ValueExpr::Field {
            meta,
            qualifier: None,
        },
        alias,
    }
}

fn push_all_source_fields(source: &Source, items: &mut Vec<SelectItem>) {
    source.for_each_field(|meta| items.push(select_item_for_meta(*meta)));
}

fn field_alias(meta: &Meta) -> Option<String> {
    (meta.api != meta.db).then(|| meta.api.to_owned())
}

fn field_ref_alias<T>(field: &FieldRef<T>) -> Option<String> {
    match &field.qualifier {
        Some(qualifier) => Some(format!("{qualifier}_{}", field.meta.api)),
        None => field_alias(field.meta),
    }
}

#[cfg(test)]
mod tests {
    use crate::typed::{
        BoolExpr, BoolOp, Field, Insert, Meta, OpSet, OrderItem, Select, SelectItem, Source,
        ValueExpr,
    };

    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static ID_FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<i32> = Field::new(&ID_META);

    fn users() -> Source {
        Source::Table {
            name: "app_users",
            alias: None,
            fields: &ID_FIELDS,
        }
    }

    #[test]
    fn subquery_value_expr_collects_nested_params_at_expression_position() {
        let subquery = crate::typed::Stmt::Select(Box::new(Select {
            ctes: Vec::new(),
            source: users(),
            joins: Vec::new(),
            distinct: false,
            distinct_on: Vec::new(),
            projection: vec![SelectItem {
                expr: ID.expr(),
                alias: None,
            }],
            filter: Some(ID.eq(10)),
            group_by: Vec::new(),
            having: None,
            order: Vec::new(),
            limit: None,
            offset: None,
            lock: None,
        }));
        let outer = ValueExpr::Subquery(Box::new(subquery));

        let mut params = Vec::new();
        outer.collect_params(&mut params);

        assert_eq!(params.len(), 1);
    }

    #[test]
    fn select_params_follow_sql_text_order() {
        let source = Source::Raw {
            sql: "select ?::int4 as id".to_owned(),
            alias: "generated".to_owned(),
            params: vec![crate::typed::Param::typed(1_i32)],
            fields: vec![ID_META],
        };
        let stmt = crate::typed::Stmt::Select(Box::new(Select {
            ctes: Vec::new(),
            source,
            joins: Vec::new(),
            distinct: false,
            distinct_on: Vec::new(),
            projection: vec![SelectItem {
                expr: ValueExpr::Param(crate::typed::Param::typed(2_i32)),
                alias: Some("projected".to_owned()),
            }],
            filter: Some(BoolExpr::Compare {
                left: ID.expr(),
                op: BoolOp::Eq,
                right: ValueExpr::Param(crate::typed::Param::typed(3_i32)),
            }),
            group_by: Vec::new(),
            having: None,
            order: vec![OrderItem::asc(ID)],
            limit: Some(crate::typed::Param::typed(10_i64)),
            offset: Some(crate::typed::Param::typed(5_i64)),
            lock: None,
        }));

        let params = stmt.params();

        assert_eq!(params.len(), 5);
        stmt.validate().unwrap();
    }

    #[test]
    fn delete_requires_filter() {
        let stmt = crate::typed::Stmt::Delete(Box::new(crate::typed::Delete {
            target: users(),
            filter: None,
            returning: Vec::new(),
        }));

        assert!(matches!(
            stmt.validate().unwrap_err(),
            crate::Error::TypedDeleteWithoutFilter
        ));
    }

    #[test]
    fn returning_all_uses_source_fields_with_api_aliases() {
        static NAME_META: Meta = Meta::new("displayName", "display_name", "text");
        static RETURN_FIELDS: [&Meta; 2] = [&ID_META, &NAME_META];
        let source = Source::Table {
            name: "public.users",
            alias: None,
            fields: &RETURN_FIELDS,
        };

        let stmt = Insert::into(source).set(ID.set(1)).returning_all();
        let built = stmt.build().unwrap();

        assert_eq!(
            built.sql,
            "INSERT INTO \"public\".\"users\" (\"id\") VALUES ($1) RETURNING \"id\", \"display_name\" AS \"displayName\""
        );
    }

    #[test]
    fn returning_all_replaces_existing_returning_fields() {
        static NAME_META: Meta = Meta::new("displayName", "display_name", "text");
        static RETURN_FIELDS: [&Meta; 2] = [&ID_META, &NAME_META];
        let source = Source::Table {
            name: "public.users",
            alias: None,
            fields: &RETURN_FIELDS,
        };

        let stmt = Insert::into(source)
            .set(ID.set(1))
            .returning(ID)
            .returning_all()
            .returning_all();
        let built = stmt.build().unwrap();

        assert_eq!(
            built.sql,
            "INSERT INTO \"public\".\"users\" (\"id\") VALUES ($1) RETURNING \"id\", \"display_name\" AS \"displayName\""
        );
    }
}
