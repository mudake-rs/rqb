use super::*;

impl SetQuery {
    pub fn new(operator: SetOperator, left: impl Into<Stmt>, right: impl Into<Stmt>) -> Self {
        Self {
            left: Box::new(left.into()),
            operator,
            right: Box::new(right.into()),
            order: Vec::new(),
            limit: None,
            offset: None,
            fetch: None,
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

    pub fn order_asc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc_nulls_first(expr));
        self
    }

    pub fn order_asc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc_nulls_last(expr));
        self
    }

    pub fn order_desc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc_nulls_first(expr));
        self
    }

    pub fn order_desc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc_nulls_last(expr));
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(Param::typed(i64::from(limit)));
        self.fetch = None;
        self
    }

    pub fn fetch_first(mut self, count: impl Into<ValueExpr>) -> Self {
        self.fetch = Some(FetchClause {
            count: count.into(),
            with_ties: false,
        });
        self.limit = None;
        self
    }

    pub fn fetch_first_with_ties(mut self, count: impl Into<ValueExpr>) -> Self {
        self.fetch = Some(FetchClause {
            count: count.into(),
            with_ties: true,
        });
        self.limit = None;
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(Param::typed(i64::from(offset)));
        self
    }

    pub fn into_source(self, alias: impl Into<String>, fields: impl Into<Vec<Meta>>) -> Source {
        subquery(self, alias, fields)
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
