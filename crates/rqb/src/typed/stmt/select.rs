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
}
