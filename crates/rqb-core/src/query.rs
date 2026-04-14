use crate::builder::SelectBuilder;
use crate::expr::{Sort, SortDir};
use crate::field::FieldRef;
use crate::request::SelectQuery;

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum QueryExpr {
    Select(Box<SelectQuery>),
    Set(Box<SetQuery>),
}

impl QueryExpr {
    pub fn union(self, right: impl Into<QueryExpr>) -> SetQuery {
        SetQuery::new(SetOperator::Union, self, right)
    }

    pub fn union_all(self, right: impl Into<QueryExpr>) -> SetQuery {
        SetQuery::new(SetOperator::UnionAll, self, right)
    }

    pub fn intersect(self, right: impl Into<QueryExpr>) -> SetQuery {
        SetQuery::new(SetOperator::Intersect, self, right)
    }

    pub fn intersect_all(self, right: impl Into<QueryExpr>) -> SetQuery {
        SetQuery::new(SetOperator::IntersectAll, self, right)
    }

    pub fn except(self, right: impl Into<QueryExpr>) -> SetQuery {
        SetQuery::new(SetOperator::Except, self, right)
    }

    pub fn except_all(self, right: impl Into<QueryExpr>) -> SetQuery {
        SetQuery::new(SetOperator::ExceptAll, self, right)
    }

    pub fn limit(mut self, limit: u32) -> Self {
        match &mut self {
            Self::Select(query) => query.request.limit = Some(limit),
            Self::Set(query) => query.limit = Some(limit),
        }
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        match &mut self {
            Self::Select(query) => query.request.offset = Some(offset),
            Self::Set(query) => query.offset = Some(offset),
        }
        self
    }
}

impl From<SelectQuery> for QueryExpr {
    fn from(value: SelectQuery) -> Self {
        Self::Select(Box::new(value))
    }
}

impl From<SelectBuilder> for QueryExpr {
    fn from(value: SelectBuilder) -> Self {
        value.build().into()
    }
}

impl From<SetQuery> for QueryExpr {
    fn from(value: SetQuery) -> Self {
        Self::Set(Box::new(value))
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct SetQuery {
    pub left: QueryExpr,
    pub operator: SetOperator,
    pub right: QueryExpr,
    pub sort: Vec<Sort>,
    pub limit: Option<u32>,
    pub offset: Option<u64>,
    pub cacheable: bool,
}

impl SetQuery {
    pub fn new(
        operator: SetOperator,
        left: impl Into<QueryExpr>,
        right: impl Into<QueryExpr>,
    ) -> Self {
        Self {
            left: left.into(),
            operator,
            right: right.into(),
            sort: Vec::new(),
            limit: None,
            offset: None,
            cacheable: true,
        }
    }

    pub fn union(self, right: impl Into<QueryExpr>) -> Self {
        SetQuery::new(SetOperator::Union, self, right)
    }

    pub fn union_all(self, right: impl Into<QueryExpr>) -> Self {
        SetQuery::new(SetOperator::UnionAll, self, right)
    }

    pub fn intersect(self, right: impl Into<QueryExpr>) -> Self {
        SetQuery::new(SetOperator::Intersect, self, right)
    }

    pub fn intersect_all(self, right: impl Into<QueryExpr>) -> Self {
        SetQuery::new(SetOperator::IntersectAll, self, right)
    }

    pub fn except(self, right: impl Into<QueryExpr>) -> Self {
        SetQuery::new(SetOperator::Except, self, right)
    }

    pub fn except_all(self, right: impl Into<QueryExpr>) -> Self {
        SetQuery::new(SetOperator::ExceptAll, self, right)
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn order_by(mut self, sort: impl Into<Sort>) -> Self {
        self.sort.push(sort.into());
        self
    }

    pub fn sort_asc(self, field: impl Into<FieldRef>) -> Self {
        self.order_by(Sort::new(field, SortDir::Asc))
    }

    pub fn sort_desc(self, field: impl Into<FieldRef>) -> Self {
        self.order_by(Sort::new(field, SortDir::Desc))
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn cacheable(mut self, cacheable: bool) -> Self {
        self.cacheable = cacheable;
        self
    }
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

impl SetOperator {
    pub fn as_sql(self) -> &'static str {
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

pub fn union(left: impl Into<QueryExpr>, right: impl Into<QueryExpr>) -> SetQuery {
    SetQuery::new(SetOperator::Union, left, right)
}

pub fn union_all(left: impl Into<QueryExpr>, right: impl Into<QueryExpr>) -> SetQuery {
    SetQuery::new(SetOperator::UnionAll, left, right)
}

pub fn intersect(left: impl Into<QueryExpr>, right: impl Into<QueryExpr>) -> SetQuery {
    SetQuery::new(SetOperator::Intersect, left, right)
}

pub fn intersect_all(left: impl Into<QueryExpr>, right: impl Into<QueryExpr>) -> SetQuery {
    SetQuery::new(SetOperator::IntersectAll, left, right)
}

pub fn except(left: impl Into<QueryExpr>, right: impl Into<QueryExpr>) -> SetQuery {
    SetQuery::new(SetOperator::Except, left, right)
}

pub fn except_all(left: impl Into<QueryExpr>, right: impl Into<QueryExpr>) -> SetQuery {
    SetQuery::new(SetOperator::ExceptAll, left, right)
}
