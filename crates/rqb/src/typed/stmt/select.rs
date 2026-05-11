use super::*;

impl Select {
    /// Creates a select statement from a root source.
    #[doc(hidden)]
    pub fn from(source: impl Into<Source>) -> Self {
        Self {
            ctes: Vec::new(),
            source: source.into(),
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

    /// Adds a CTE to the select.
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Turns this select into a subquery source by inferring exposed fields.
    ///
    /// Inference succeeds for default projection or plain field projections.
    /// Use `into_source(alias, fields)` when projecting computed expressions or
    /// renaming exposed columns.
    pub fn try_into_source(self, alias: impl Into<String>) -> crate::Result<Source> {
        let fields = self.inferred_source_fields()?;
        Ok(subquery(self, alias, fields))
    }

    /// Turns this select into a subquery source with explicit exposed fields.
    pub fn into_source(self, alias: impl Into<String>, fields: impl IntoFieldMetas) -> Source {
        subquery(self, alias, fields)
    }

    /// Turns this select into a CTE by inferring exposed fields.
    ///
    /// Inference has the same rules as `try_into_source`.
    pub fn try_into_cte(self, name: impl Into<String>) -> crate::Result<Cte> {
        let fields = self.inferred_source_fields()?;
        Ok(cte(name, self, fields))
    }

    /// Turns this select into a CTE with explicit exposed fields.
    pub fn into_cte(self, name: impl Into<String>, fields: impl IntoFieldMetas) -> Cte {
        cte(name, self, fields)
    }

    /// Adds a schema field or aliased field reference to the projection.
    ///
    /// Plain fields keep their database/API alias rules. Qualified fields get
    /// stable aliases such as `u_email`, which makes `sqlx::FromRow` mapping
    /// deterministic for joins.
    pub fn column(mut self, field: impl Into<SelectItem>) -> Self {
        self.projection.push(field.into());
        self
    }

    /// Adds multiple schema fields or aliased field references to the projection.
    pub fn columns(mut self, fields: impl IntoSelectItems) -> Self {
        self.projection.extend(fields.into_select_items());
        self
    }

    /// Adds an expression without an output alias.
    pub fn expr(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.projection.push(SelectItem {
            expr: expr.into(),
            alias: None,
        });
        self
    }

    /// Adds multiple expressions without output aliases.
    ///
    /// This accepts regular iterators of value expressions. For heterogeneous
    /// field or aliased item batches, use [`Select::columns`] or
    /// [`Select::items`] with tuple syntax.
    pub fn exprs<I, T>(mut self, exprs: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<ValueExpr>,
    {
        self.projection
            .extend(exprs.into_iter().map(|expr| SelectItem {
                expr: expr.into(),
                alias: None,
            }));
        self
    }

    /// Adds a fully specified projection item, usually an aliased expression.
    pub fn item(mut self, item: SelectItem) -> Self {
        self.projection.push(item);
        self
    }

    /// Adds multiple fully specified projection items.
    pub fn items(mut self, items: impl IntoSelectItems) -> Self {
        self.projection.extend(items.into_select_items());
        self
    }

    /// Alias for `item(...)` in aggregate-heavy selects.
    pub fn agg(self, item: SelectItem) -> Self {
        self.item(item)
    }

    /// Adds multiple aggregate projection items.
    pub fn aggs(self, items: impl IntoSelectItems) -> Self {
        self.items(items)
    }

    /// Adds a predicate to `WHERE`, composing with existing predicates using `AND`.
    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(BoolExpr::and_option(self.filter, filter));
        self
    }

    /// Adds a predicate to `WHERE`, composing with existing predicates using `OR`.
    ///
    /// Use `filter(or([...]))` when only part of the current `WHERE` tree
    /// should be OR-grouped.
    pub fn or_filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(BoolExpr::or_option(self.filter, filter));
        self
    }

    /// Replaces the entire `WHERE` predicate.
    pub fn replace_filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Adds a predicate only when `condition` is true.
    pub fn filter_if(self, condition: bool, filter: BoolExpr) -> Self {
        if condition { self.filter(filter) } else { self }
    }

    /// Adds a predicate built from an optional value.
    pub fn filter_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> BoolExpr) -> Self {
        match value {
            Some(value) => self.filter(f(value)),
            None => self,
        }
    }

    /// Adds an OR-composed predicate only when `condition` is true.
    pub fn or_filter_if(self, condition: bool, filter: BoolExpr) -> Self {
        if condition {
            self.or_filter(filter)
        } else {
            self
        }
    }

    /// Adds an OR-composed predicate built from an optional value.
    pub fn or_filter_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> BoolExpr) -> Self {
        match value {
            Some(value) => self.or_filter(f(value)),
            None => self,
        }
    }

    /// Applies an arbitrary builder transformation.
    pub fn apply(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }

    /// Adds `DISTINCT`.
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Adds a `DISTINCT ON` expression.
    pub fn distinct_on(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.distinct_on.push(expr.into());
        self
    }

    /// Adds an inner join.
    pub fn join(mut self, source: impl Into<Source>, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Inner, source, on));
        self
    }

    /// Adds a left join.
    pub fn left_join(mut self, source: impl Into<Source>, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Left, source, on));
        self
    }

    /// Adds a right join.
    pub fn right_join(mut self, source: impl Into<Source>, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Right, source, on));
        self
    }

    /// Adds a full join.
    pub fn full_join(mut self, source: impl Into<Source>, on: BoolExpr) -> Self {
        self.joins.push(Join::new(JoinKind::Full, source, on));
        self
    }

    /// Adds an inner lateral join.
    pub fn join_lateral(mut self, source: impl Into<Source>, on: BoolExpr) -> Self {
        self.joins.push(Join::lateral(JoinKind::Inner, source, on));
        self
    }

    /// Adds a left lateral join.
    pub fn left_join_lateral(mut self, source: impl Into<Source>, on: BoolExpr) -> Self {
        self.joins.push(Join::lateral(JoinKind::Left, source, on));
        self
    }

    /// Adds a cross join.
    pub fn cross_join(mut self, source: impl Into<Source>) -> Self {
        self.joins.push(Join::cross(source));
        self
    }

    /// Adds a lateral cross join.
    pub fn cross_join_lateral(mut self, source: impl Into<Source>) -> Self {
        self.joins.push(Join::cross_lateral(source));
        self
    }

    /// Adds a fully specified `ORDER BY` item.
    pub fn order_by(mut self, item: OrderItem) -> Self {
        self.order.push(item);
        self
    }

    /// Adds a `GROUP BY` expression.
    pub fn group_by(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.group_by.push(GroupByItem::expr(expr));
        self
    }

    /// Adds a `ROLLUP` grouping item.
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

    /// Adds a `CUBE` grouping item.
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

    /// Adds a `GROUPING SETS` item.
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

    /// Adds a `HAVING` predicate, composing with existing predicates using `AND`.
    pub fn having(mut self, having: BoolExpr) -> Self {
        self.having = Some(BoolExpr::and_option(self.having, having));
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

    /// Adds a row lock with the given mode.
    pub fn lock(mut self, mode: LockMode) -> Self {
        self.lock = Some(RowLock::new(mode));
        self
    }

    /// Adds a row lock scoped to a relation alias.
    pub fn lock_of(mut self, mode: LockMode, relation: impl Into<String>) -> Self {
        self.lock = Some(RowLock::new(mode).of(relation));
        self
    }

    /// Adds `FOR UPDATE`.
    pub fn for_update(self) -> Self {
        self.lock(LockMode::Update)
    }

    /// Adds `FOR UPDATE OF relation`.
    pub fn for_update_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::Update, relation)
    }

    /// Adds `FOR NO KEY UPDATE`.
    pub fn for_no_key_update(self) -> Self {
        self.lock(LockMode::NoKeyUpdate)
    }

    /// Adds `FOR NO KEY UPDATE OF relation`.
    pub fn for_no_key_update_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::NoKeyUpdate, relation)
    }

    /// Adds `FOR SHARE`.
    pub fn for_share(self) -> Self {
        self.lock(LockMode::Share)
    }

    /// Adds `FOR SHARE OF relation`.
    pub fn for_share_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::Share, relation)
    }

    /// Adds `FOR KEY SHARE`.
    pub fn for_key_share(self) -> Self {
        self.lock(LockMode::KeyShare)
    }

    /// Adds `FOR KEY SHARE OF relation`.
    pub fn for_key_share_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::KeyShare, relation)
    }

    /// Adds a relation alias to the current row lock, creating `FOR UPDATE` if absent.
    pub fn lock_relation(mut self, relation: impl Into<String>) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().of(relation));
        self
    }

    /// Sets `NOWAIT` on the current row lock, creating `FOR UPDATE` if absent.
    pub fn nowait(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().nowait());
        self
    }

    /// Sets `SKIP LOCKED` on the current row lock, creating `FOR UPDATE` if absent.
    pub fn skip_locked(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().skip_locked());
        self
    }

    pub(crate) fn inferred_source_fields(&self) -> crate::Result<Vec<Meta>> {
        if self.projection.is_empty() {
            let mut fields = Vec::new();
            self.source.for_each_field(|field| fields.push(*field));
            return Ok(fields);
        }

        let mut fields = Vec::with_capacity(self.projection.len());
        for item in &self.projection {
            let Some(meta) = item.expr.field_meta() else {
                return Err(crate::Error::InvalidSelectShape {
                    message: "try_into_source cannot infer fields from computed projection; use into_source",
                });
            };
            if item.alias.as_deref().is_some_and(|alias| alias != meta.db) {
                return Err(crate::Error::InvalidSelectShape {
                    message: "try_into_source cannot infer fields from aliased projection; use into_source",
                });
            }
            fields.push(*meta);
        }
        Ok(fields)
    }
}
