use super::*;

impl SelectItem {
    pub fn new(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            alias: None,
        }
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

impl<T> From<Field<T>> for SelectItem {
    fn from(field: Field<T>) -> Self {
        select_item_for_field(field)
    }
}

impl<T> From<FieldRef<T>> for SelectItem {
    fn from(field: FieldRef<T>) -> Self {
        select_item_for_ref(field)
    }
}

impl Assignment {
    pub fn new<T>(field: Field<T>, value: impl Into<ValueExpr>) -> Self {
        Self {
            field: *field.meta,
            value: value.into(),
        }
    }
}

impl OrderDirection {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl NullsPosition {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::First => "NULLS FIRST",
            Self::Last => "NULLS LAST",
        }
    }
}

impl LockMode {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Update => "FOR UPDATE",
            Self::NoKeyUpdate => "FOR NO KEY UPDATE",
            Self::Share => "FOR SHARE",
            Self::KeyShare => "FOR KEY SHARE",
        }
    }
}

impl LockWait {
    pub const fn as_sql(self) -> Option<&'static str> {
        match self {
            Self::Wait => None,
            Self::NoWait => Some("NOWAIT"),
            Self::SkipLocked => Some("SKIP LOCKED"),
        }
    }
}

impl RowLock {
    pub const fn new(mode: LockMode) -> Self {
        Self {
            mode,
            wait: LockWait::Wait,
            of: Vec::new(),
        }
    }

    pub const fn nowait(mut self) -> Self {
        self.wait = LockWait::NoWait;
        self
    }

    pub const fn skip_locked(mut self) -> Self {
        self.wait = LockWait::SkipLocked;
        self
    }

    pub fn of(mut self, relation: impl Into<String>) -> Self {
        self.of.push(relation.into());
        self
    }
}

impl Default for RowLock {
    fn default() -> Self {
        Self::new(LockMode::Update)
    }
}

impl OrderItem {
    pub fn asc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Asc,
            nulls: None,
        }
    }

    pub fn desc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Desc,
            nulls: None,
        }
    }

    pub fn nulls_first(mut self) -> Self {
        self.nulls = Some(NullsPosition::First);
        self
    }

    pub fn nulls_last(mut self) -> Self {
        self.nulls = Some(NullsPosition::Last);
        self
    }

    pub fn asc_nulls_first(expr: impl Into<ValueExpr>) -> Self {
        Self::asc(expr).nulls_first()
    }

    pub fn asc_nulls_last(expr: impl Into<ValueExpr>) -> Self {
        Self::asc(expr).nulls_last()
    }

    pub fn desc_nulls_first(expr: impl Into<ValueExpr>) -> Self {
        Self::desc(expr).nulls_first()
    }

    pub fn desc_nulls_last(expr: impl Into<ValueExpr>) -> Self {
        Self::desc(expr).nulls_last()
    }
}

impl GroupByItem {
    pub fn expr(expr: impl Into<ValueExpr>) -> Self {
        Self::Expr(expr.into())
    }

    pub fn rollup(exprs: impl IntoIterator<Item = ValueExpr>) -> Self {
        Self::Rollup(exprs.into_iter().collect())
    }

    pub fn cube(exprs: impl IntoIterator<Item = ValueExpr>) -> Self {
        Self::Cube(exprs.into_iter().collect())
    }

    pub fn grouping_sets(sets: impl IntoIterator<Item = Vec<ValueExpr>>) -> Self {
        Self::GroupingSets(sets.into_iter().collect())
    }
}
