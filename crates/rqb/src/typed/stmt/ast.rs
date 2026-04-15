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
    pub(super) insert: Insert,
    pub(super) target: ConflictTarget,
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
