use super::*;

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
