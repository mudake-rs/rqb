use super::*;

impl SetQuery {
    /// Creates a set query from two statements.
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

    /// Chains a `UNION` set operation.
    pub fn union(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::Union, self, right)
    }

    /// Chains a `UNION ALL` set operation.
    pub fn union_all(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::UnionAll, self, right)
    }

    /// Chains an `INTERSECT` set operation.
    pub fn intersect(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::Intersect, self, right)
    }

    /// Chains an `INTERSECT ALL` set operation.
    pub fn intersect_all(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::IntersectAll, self, right)
    }

    /// Chains an `EXCEPT` set operation.
    pub fn except(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::Except, self, right)
    }

    /// Chains an `EXCEPT ALL` set operation.
    pub fn except_all(self, right: impl Into<Stmt>) -> Self {
        Self::new(SetOperator::ExceptAll, self, right)
    }

    /// Adds a fully specified `ORDER BY` item.
    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order.push(item);
        self
    }

    /// Adds ascending order.
    pub fn order_asc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc(expr));
        self
    }

    /// Adds descending order.
    pub fn order_desc(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc(expr));
        self
    }

    /// Adds ascending order with `NULLS FIRST`.
    pub fn order_asc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc_nulls_first(expr));
        self
    }

    /// Adds ascending order with `NULLS LAST`.
    pub fn order_asc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::asc_nulls_last(expr));
        self
    }

    /// Adds descending order with `NULLS FIRST`.
    pub fn order_desc_nulls_first(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc_nulls_first(expr));
        self
    }

    /// Adds descending order with `NULLS LAST`.
    pub fn order_desc_nulls_last(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.order.push(OrderItem::desc_nulls_last(expr));
        self
    }

    /// Sets a `LIMIT` value and clears any `FETCH FIRST` clause.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(Param::typed(i64::from(limit)));
        self.fetch = None;
        self
    }

    /// Sets a `FETCH FIRST` clause and clears any `LIMIT`.
    pub fn fetch_first(mut self, count: impl Into<ValueExpr>) -> Self {
        self.fetch = Some(FetchClause {
            count: count.into(),
            with_ties: false,
        });
        self.limit = None;
        self
    }

    /// Sets `FETCH FIRST ... WITH TIES` and clears any `LIMIT`.
    pub fn fetch_first_with_ties(mut self, count: impl Into<ValueExpr>) -> Self {
        self.fetch = Some(FetchClause {
            count: count.into(),
            with_ties: true,
        });
        self.limit = None;
        self
    }

    /// Sets an `OFFSET` value.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(Param::typed(i64::from(offset)));
        self
    }

    /// Turns this set query into a subquery source with explicit fields.
    pub fn into_source(self, alias: impl Into<String>, fields: impl IntoFieldMetas) -> Source {
        subquery(self, alias, fields)
    }
}

impl Select {
    /// Builds a `UNION` with this select as the left side.
    pub fn union(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::Union, self, right)
    }

    /// Builds a `UNION ALL` with this select as the left side.
    pub fn union_all(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::UnionAll, self, right)
    }

    /// Builds an `INTERSECT` with this select as the left side.
    pub fn intersect(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::Intersect, self, right)
    }

    /// Builds an `INTERSECT ALL` with this select as the left side.
    pub fn intersect_all(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::IntersectAll, self, right)
    }

    /// Builds an `EXCEPT` with this select as the left side.
    pub fn except(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::Except, self, right)
    }

    /// Builds an `EXCEPT ALL` with this select as the left side.
    pub fn except_all(self, right: impl Into<Stmt>) -> SetQuery {
        SetQuery::new(SetOperator::ExceptAll, self, right)
    }
}
