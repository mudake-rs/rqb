use super::*;

impl SelectItem {
    /// Creates an unaliased projection item.
    pub fn new(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            alias: None,
        }
    }

    /// Sets the SQL alias for this projection item.
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

/// Converts fields, metadata, and tuples of either into projection items.
///
/// This lets heterogeneous field projections use tuple syntax, for example
/// `select(users::table()).columns((users::ID, users::EMAIL))`.
#[doc(hidden)]
pub trait IntoSelectItems {
    /// Converts this value into projection items.
    fn into_select_items(self) -> Vec<SelectItem>;
}

impl<T> From<Field<T>> for SelectItem {
    fn from(field: Field<T>) -> Self {
        select_item_for_field(field)
    }
}

impl<T> From<&Field<T>> for SelectItem {
    fn from(field: &Field<T>) -> Self {
        select_item_for_field(*field)
    }
}

impl<T> From<FieldRef<T>> for SelectItem {
    fn from(field: FieldRef<T>) -> Self {
        select_item_for_ref(field)
    }
}

impl<T> From<&FieldRef<T>> for SelectItem {
    fn from(field: &FieldRef<T>) -> Self {
        select_item_for_ref(field.clone())
    }
}

impl From<Meta> for SelectItem {
    fn from(meta: Meta) -> Self {
        select_item_for_meta(meta)
    }
}

impl From<&Meta> for SelectItem {
    fn from(meta: &Meta) -> Self {
        select_item_for_meta(*meta)
    }
}

impl IntoSelectItems for SelectItem {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self]
    }
}

impl<T> IntoSelectItems for Field<T> {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self.into()]
    }
}

impl<T> IntoSelectItems for &Field<T> {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self.into()]
    }
}

impl<T> IntoSelectItems for FieldRef<T> {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self.into()]
    }
}

impl<T> IntoSelectItems for &FieldRef<T> {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self.into()]
    }
}

impl IntoSelectItems for Meta {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self.into()]
    }
}

impl IntoSelectItems for &Meta {
    fn into_select_items(self) -> Vec<SelectItem> {
        vec![self.into()]
    }
}

impl IntoSelectItems for Vec<SelectItem> {
    fn into_select_items(self) -> Vec<SelectItem> {
        self
    }
}

impl IntoSelectItems for &[SelectItem] {
    fn into_select_items(self) -> Vec<SelectItem> {
        self.to_vec()
    }
}

impl<const N: usize> IntoSelectItems for [SelectItem; N] {
    fn into_select_items(self) -> Vec<SelectItem> {
        self.into_iter().collect()
    }
}

macro_rules! impl_select_item_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> IntoSelectItems for ($($name,)+)
        where
            $($name: IntoSelectItems,)+
        {
            #[allow(non_snake_case)]
            fn into_select_items(self) -> Vec<SelectItem> {
                let ($($name,)+) = self;
                let mut items = Vec::new();
                $(items.extend($name.into_select_items());)+
                items
            }
        }
    };
}

impl_select_item_tuple!(A, B);
impl_select_item_tuple!(A, B, C);
impl_select_item_tuple!(A, B, C, D);
impl_select_item_tuple!(A, B, C, D, E);
impl_select_item_tuple!(A, B, C, D, E, F);
impl_select_item_tuple!(A, B, C, D, E, F, G);
impl_select_item_tuple!(A, B, C, D, E, F, G, H);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_select_item_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

impl Assignment {
    /// Creates a field assignment from a value expression.
    pub fn new<T>(field: Field<T>, value: impl Into<ValueExpr>) -> Self {
        Self {
            field: *field.meta,
            value: crate::AssignmentValue::Expr(value.into()),
        }
    }
}

impl OrderDirection {
    /// Returns the SQL token for this order direction.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl NullsPosition {
    /// Returns the SQL token for this null placement.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::First => "NULLS FIRST",
            Self::Last => "NULLS LAST",
        }
    }
}

impl LockMode {
    /// Returns the SQL token for this lock mode.
    #[inline]
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
    /// Returns the optional SQL token for this lock wait behavior.
    #[inline]
    pub const fn as_sql(self) -> Option<&'static str> {
        match self {
            Self::Wait => None,
            Self::NoWait => Some("NOWAIT"),
            Self::SkipLocked => Some("SKIP LOCKED"),
        }
    }
}

impl RowLock {
    /// Creates a row lock clause with default wait behavior.
    #[inline]
    pub const fn new(mode: LockMode) -> Self {
        Self {
            mode,
            wait: LockWait::Wait,
            of: Vec::new(),
        }
    }

    /// Sets `NOWAIT`.
    #[inline]
    pub const fn nowait(mut self) -> Self {
        self.wait = LockWait::NoWait;
        self
    }

    /// Sets `SKIP LOCKED`.
    #[inline]
    pub const fn skip_locked(mut self) -> Self {
        self.wait = LockWait::SkipLocked;
        self
    }

    /// Restricts the lock to a relation alias.
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
    /// Creates ascending order for an expression.
    pub fn asc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Asc,
            nulls: None,
        }
    }

    /// Creates descending order for an expression.
    pub fn desc(expr: impl Into<ValueExpr>) -> Self {
        Self {
            expr: expr.into(),
            direction: OrderDirection::Desc,
            nulls: None,
        }
    }

    /// Adds `NULLS FIRST`.
    #[inline]
    pub fn nulls_first(mut self) -> Self {
        self.nulls = Some(NullsPosition::First);
        self
    }

    /// Adds `NULLS LAST`.
    #[inline]
    pub fn nulls_last(mut self) -> Self {
        self.nulls = Some(NullsPosition::Last);
        self
    }

    /// Creates ascending order with `NULLS FIRST`.
    pub fn asc_nulls_first(expr: impl Into<ValueExpr>) -> Self {
        Self::asc(expr).nulls_first()
    }

    /// Creates ascending order with `NULLS LAST`.
    pub fn asc_nulls_last(expr: impl Into<ValueExpr>) -> Self {
        Self::asc(expr).nulls_last()
    }

    /// Creates descending order with `NULLS FIRST`.
    pub fn desc_nulls_first(expr: impl Into<ValueExpr>) -> Self {
        Self::desc(expr).nulls_first()
    }

    /// Creates descending order with `NULLS LAST`.
    pub fn desc_nulls_last(expr: impl Into<ValueExpr>) -> Self {
        Self::desc(expr).nulls_last()
    }
}

impl GroupByItem {
    /// Creates a regular `GROUP BY` expression.
    pub fn expr(expr: impl Into<ValueExpr>) -> Self {
        Self::Expr(expr.into())
    }

    /// Creates a `ROLLUP` group item.
    pub fn rollup(exprs: impl IntoIterator<Item = ValueExpr>) -> Self {
        Self::Rollup(exprs.into_iter().collect())
    }

    /// Creates a `CUBE` group item.
    pub fn cube(exprs: impl IntoIterator<Item = ValueExpr>) -> Self {
        Self::Cube(exprs.into_iter().collect())
    }

    /// Creates a `GROUPING SETS` item.
    pub fn grouping_sets(sets: impl IntoIterator<Item = Vec<ValueExpr>>) -> Self {
        Self::GroupingSets(sets.into_iter().collect())
    }
}
