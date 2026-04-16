use super::*;

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
            fetch: None,
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
        self.filter = Some(BoolExpr::and_option(self.filter, filter));
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
        self.group_by.push(GroupByItem::expr(expr));
        self
    }

    pub fn rollup<I, E>(mut self, exprs: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<ValueExpr>,
    {
        self.group_by.push(GroupByItem::Rollup(
            exprs.into_iter().map(Into::into).collect(),
        ));
        self
    }

    pub fn cube<I, E>(mut self, exprs: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<ValueExpr>,
    {
        self.group_by.push(GroupByItem::Cube(
            exprs.into_iter().map(Into::into).collect(),
        ));
        self
    }

    pub fn grouping_sets<I, S, E>(mut self, sets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: IntoIterator<Item = E>,
        E: Into<ValueExpr>,
    {
        self.group_by.push(GroupByItem::GroupingSets(
            sets.into_iter()
                .map(|set| set.into_iter().map(Into::into).collect())
                .collect(),
        ));
        self
    }

    pub fn having(mut self, having: BoolExpr) -> Self {
        self.having = Some(BoolExpr::and_option(self.having, having));
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

    pub fn lock(mut self, mode: LockMode) -> Self {
        self.lock = Some(RowLock::new(mode));
        self
    }

    pub fn lock_of(mut self, mode: LockMode, relation: impl Into<String>) -> Self {
        self.lock = Some(RowLock::new(mode).of(relation));
        self
    }

    pub fn for_update(self) -> Self {
        self.lock(LockMode::Update)
    }

    pub fn for_update_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::Update, relation)
    }

    pub fn for_no_key_update(self) -> Self {
        self.lock(LockMode::NoKeyUpdate)
    }

    pub fn for_no_key_update_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::NoKeyUpdate, relation)
    }

    pub fn for_share(self) -> Self {
        self.lock(LockMode::Share)
    }

    pub fn for_share_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::Share, relation)
    }

    pub fn for_key_share(self) -> Self {
        self.lock(LockMode::KeyShare)
    }

    pub fn for_key_share_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::KeyShare, relation)
    }

    pub fn lock_relation(mut self, relation: impl Into<String>) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().of(relation));
        self
    }

    pub fn nowait(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().nowait());
        self
    }

    pub fn skip_locked(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().skip_locked());
        self
    }
}
