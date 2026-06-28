use super::*;

/// Starts a typed `SELECT` statement.
pub fn select(source: impl Into<Source>) -> Select {
    Select::from(source)
}

/// Starts a typed `INSERT` statement.
pub fn insert(target: impl Into<Source>) -> Insert {
    Insert::into(target)
}

/// Starts a typed `UPDATE` statement.
pub fn update(target: impl Into<Source>) -> Update {
    Update::table(target)
}

/// Starts a typed `DELETE FROM` statement.
pub fn delete_from(target: impl Into<Source>) -> Delete {
    Delete::from(target)
}

/// Starts a typed PostgreSQL `MERGE` statement.
pub fn merge_into(target: impl Into<Source>, using: impl Into<Source>, on: BoolExpr) -> Merge {
    Merge::into(target, using, on)
}

/// Starts a server-owned raw SQL statement.
///
/// Raw SQL is still parameterized: use `?` placeholders with
/// [`RawStmt::bind`](crate::RawStmt::bind), and `??` when the SQL text needs a
/// literal question mark.
pub fn raw(sql: impl Into<String>) -> RawStmt {
    RawStmt::new(sql)
}

/// Builds a `UNION` set query.
pub fn union(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::Union, left, right)
}

/// Builds a `UNION ALL` set query.
pub fn union_all(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::UnionAll, left, right)
}

/// Builds an `INTERSECT` set query.
pub fn intersect(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::Intersect, left, right)
}

/// Builds an `INTERSECT ALL` set query.
pub fn intersect_all(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::IntersectAll, left, right)
}

/// Builds an `EXCEPT` set query.
pub fn except(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::Except, left, right)
}

/// Builds an `EXCEPT ALL` set query.
pub fn except_all(left: impl Into<Stmt>, right: impl Into<Stmt>) -> SetQuery {
    SetQuery::new(SetOperator::ExceptAll, left, right)
}

impl Stmt {
    /// Creates a raw SQL statement variant.
    ///
    /// Prefer the free [`raw`] constructor for normal use; this exists for code
    /// that is already working with the enum form.
    pub fn raw(sql: impl Into<String>) -> Self {
        Self::Raw(Box::new(RawStmt::new(sql)))
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

impl From<Insert> for Stmt {
    fn from(insert: Insert) -> Self {
        Self::Insert(Box::new(insert))
    }
}

impl From<Update> for Stmt {
    fn from(update: Update) -> Self {
        Self::Update(Box::new(update))
    }
}

impl From<Delete> for Stmt {
    fn from(delete: Delete) -> Self {
        Self::Delete(Box::new(delete))
    }
}

impl From<RawStmt> for Stmt {
    fn from(raw: RawStmt) -> Self {
        Self::Raw(Box::new(raw))
    }
}

impl SetOperator {
    /// Returns the SQL operator token.
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
