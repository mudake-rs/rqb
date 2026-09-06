use super::*;

impl Select {
    /// Creates a select statement from a root source.
    pub(crate) fn from(source: impl Into<Source>) -> Self {
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
            row_limit: None,
            offset: None,
            lock: None,
        }
    }

    /// Adds a CTE to the select.
    #[inline]
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Turns this select into a subquery source by inferring exposed fields.
    ///
    /// Inference succeeds for default projection or plain field projections.
    /// Use `into_source(alias, fields)` when projecting computed expressions or
    /// renaming exposed columns. Inferred database and API names must be unique.
    pub fn infer_source(self, alias: impl Into<String>) -> crate::Result<Source> {
        let fields = self.inferred_source_fields()?;
        Ok(subquery(self, alias, fields))
    }

    /// Turns this select into a subquery source with explicit exposed fields.
    pub fn into_source(self, alias: impl Into<String>, fields: impl IntoFieldMetas) -> Source {
        subquery(self, alias, fields)
    }

    /// Turns this select into a CTE by inferring exposed fields.
    ///
    /// Inference has the same rules as `infer_source`.
    pub fn infer_cte(self, name: impl Into<String>) -> crate::Result<Cte> {
        let fields = self.inferred_source_fields()?;
        Ok(cte(name, self, fields))
    }

    /// Turns this select into a CTE with explicit exposed fields.
    pub fn into_cte(self, name: impl Into<String>, fields: impl IntoFieldMetas) -> Cte {
        cte(name, self, fields)
    }

    /// Adds a schema field or qualified field reference to the projection.
    ///
    /// Plain fields keep their database/API alias rules. Qualified fields get
    /// stable aliases such as `u_email`, which makes `sqlx::FromRow` mapping
    /// deterministic for joins.
    pub fn column(mut self, field: impl IntoColumn) -> Self {
        let mut columns = ColumnList {
            items: self.projection,
        };
        field.push_column(&mut columns);
        self.projection = columns.items;
        self
    }

    /// Adds multiple schema fields or qualified field references to the projection.
    pub fn columns(mut self, fields: impl IntoColumns) -> Self {
        self.projection.extend(fields.into_columns().items);
        self
    }

    /// Adds the root source fields that default projection would render.
    ///
    /// This is useful when a query wants the normal schema-driven row plus
    /// computed projection items. It only expands the root source fields; joined
    /// fields still need explicit projection.
    pub fn default_columns(mut self) -> Self {
        let qualifier = self.source.explicit_alias();
        self.source.for_each_field(|meta| {
            self.projection
                .push(select_item_for_source_meta(*meta, qualifier));
        });
        self
    }

    /// Adds an expression without an output alias.
    ///
    /// Adding any projection item switches the select out of default root-field
    /// projection. Use [`Select::default_columns`] first when you want the
    /// default row plus computed expressions.
    pub fn expr(mut self, expr: impl Into<ValueExpr>) -> Self {
        self.projection.push(SelectItem {
            expr: expr.into(),
            alias: None,
        });
        self
    }

    /// Adds an expression with an output alias.
    ///
    /// Use this for computed columns and deliberate field renames. Adding any
    /// projection item switches the select out of default root-field
    /// projection. Use [`Select::default_columns`] first when you want the
    /// default row plus computed expressions.
    pub fn expr_as(mut self, expr: impl Into<ValueExpr>, alias: impl Into<String>) -> Self {
        self.projection.push(SelectItem {
            expr: expr.into(),
            alias: Some(alias.into()),
        });
        self
    }

    /// Applies an arbitrary builder transformation.
    pub fn apply(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }

    /// Adds `DISTINCT`.
    #[inline]
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
    #[inline]
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
        self.group_by
            .push(GroupByItem::rollup(exprs.into_iter().map(Into::into)));
        self
    }

    /// Adds a `CUBE` grouping item.
    pub fn cube<I, E>(mut self, exprs: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<ValueExpr>,
    {
        self.group_by
            .push(GroupByItem::cube(exprs.into_iter().map(Into::into)));
        self
    }

    /// Adds a `GROUPING SETS` item.
    pub fn grouping_sets<I, S, E>(mut self, sets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: IntoIterator<Item = E>,
        E: Into<ValueExpr>,
    {
        self.group_by.push(GroupByItem::grouping_sets(
            sets.into_iter()
                .map(|set| set.into_iter().map(Into::into).collect()),
        ));
        self
    }

    /// Adds a `HAVING` predicate, composing with existing predicates using `AND`.
    #[inline]
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

    /// Sets a `LIMIT` value.
    #[inline]
    pub fn limit(mut self, limit: u32) -> Self {
        self.row_limit = Some(RowLimit::Limit(Param::typed(i64::from(limit))));
        self
    }

    /// Sets a `FETCH FIRST` clause.
    pub fn fetch_first(mut self, count: impl Into<ValueExpr>) -> Self {
        self.row_limit = Some(RowLimit::Fetch(FetchClause {
            count: count.into(),
            with_ties: false,
        }));
        self
    }

    /// Sets `FETCH FIRST ... WITH TIES`.
    pub fn fetch_first_with_ties(mut self, count: impl Into<ValueExpr>) -> Self {
        self.row_limit = Some(RowLimit::Fetch(FetchClause {
            count: count.into(),
            with_ties: true,
        }));
        self
    }

    /// Sets an `OFFSET` value.
    #[inline]
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(Param::typed(i64::from(offset)));
        self
    }

    /// Adds a row lock with the given mode.
    #[inline]
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
    #[inline]
    pub fn for_update(self) -> Self {
        self.lock(LockMode::Update)
    }

    /// Adds `FOR UPDATE OF relation`.
    pub fn for_update_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::Update, relation)
    }

    /// Adds `FOR NO KEY UPDATE`.
    #[inline]
    pub fn for_no_key_update(self) -> Self {
        self.lock(LockMode::NoKeyUpdate)
    }

    /// Adds `FOR NO KEY UPDATE OF relation`.
    pub fn for_no_key_update_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::NoKeyUpdate, relation)
    }

    /// Adds `FOR SHARE`.
    #[inline]
    pub fn for_share(self) -> Self {
        self.lock(LockMode::Share)
    }

    /// Adds `FOR SHARE OF relation`.
    pub fn for_share_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::Share, relation)
    }

    /// Adds `FOR KEY SHARE`.
    #[inline]
    pub fn for_key_share(self) -> Self {
        self.lock(LockMode::KeyShare)
    }

    /// Adds `FOR KEY SHARE OF relation`.
    pub fn for_key_share_of(self, relation: impl Into<String>) -> Self {
        self.lock_of(LockMode::KeyShare, relation)
    }

    /// Sets `NOWAIT` on the current row lock, creating `FOR UPDATE` if absent.
    #[inline]
    pub fn nowait(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().nowait());
        self
    }

    /// Sets `SKIP LOCKED` on the current row lock, creating `FOR UPDATE` if absent.
    #[inline]
    pub fn skip_locked(mut self) -> Self {
        self.lock = Some(self.lock.unwrap_or_default().skip_locked());
        self
    }

    pub(crate) fn inferred_source_fields(&self) -> crate::Result<Vec<Meta>> {
        let mut fields = Vec::with_capacity(self.projection.len());
        if self.projection.is_empty() {
            self.source.for_each_field(|field| fields.push(*field));
        }
        for item in &self.projection {
            let ValueExpr::Field { meta, qualifier } = &item.expr else {
                return Err(crate::Error::InvalidSelectShape {
                    message: "infer_source cannot infer fields from computed projection; use into_source",
                });
            };
            let generated_alias = field_ref_alias(meta, qualifier.as_deref());
            if item
                .alias
                .as_deref()
                .is_some_and(|alias| alias != meta.db && Some(alias) != generated_alias.as_deref())
            {
                return Err(crate::Error::InvalidSelectShape {
                    message: "infer_source cannot infer fields from aliased projection; use into_source",
                });
            }
            fields.push(*meta);
        }
        for (index, field) in fields.iter().enumerate() {
            if fields[..index]
                .iter()
                .any(|other| other.db == field.db || other.api == field.api)
            {
                return Err(crate::Error::InvalidSelectShape {
                    message: "inferred fields must have unique database and API names; use into_source or into_cte with distinct metadata",
                });
            }
        }
        Ok(fields)
    }
}

impl_filter_methods!(Select);
