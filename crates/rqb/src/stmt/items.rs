use super::*;

/// Opaque column-list container used by [`Select::columns`](crate::Select::columns).
#[doc(hidden)]
#[derive(Clone, Debug)]
#[must_use]
pub struct ColumnList {
    pub(crate) items: Vec<SelectItem>,
}

impl ColumnList {
    pub(crate) fn one(item: SelectItem) -> Self {
        Self { items: vec![item] }
    }
}

/// Converts one field, field reference, or metadata value into one column projection.
#[doc(hidden)]
pub trait IntoColumn {
    /// Converts this value into one column projection.
    fn into_column(self) -> ColumnList;
}

/// Converts fields, metadata, and tuples of either into column projections.
///
/// This lets heterogeneous field projections use tuple syntax, for example
/// `select(users::table()).columns((users::ID, users::EMAIL))`.
#[doc(hidden)]
pub trait IntoColumns {
    /// Converts this value into column projections.
    fn into_columns(self) -> ColumnList;
}

impl<T> IntoColumn for Field<T> {
    fn into_column(self) -> ColumnList {
        ColumnList::one(select_item_for_field(self))
    }
}

impl<T> IntoColumn for &Field<T> {
    fn into_column(self) -> ColumnList {
        ColumnList::one(select_item_for_field(*self))
    }
}

impl<T> IntoColumn for FieldRef<T> {
    fn into_column(self) -> ColumnList {
        ColumnList::one(select_item_for_ref(self))
    }
}

impl<T> IntoColumn for &FieldRef<T> {
    fn into_column(self) -> ColumnList {
        ColumnList::one(select_item_for_ref(self.clone()))
    }
}

impl IntoColumn for Meta {
    fn into_column(self) -> ColumnList {
        ColumnList::one(select_item_for_meta(self))
    }
}

impl IntoColumn for &Meta {
    fn into_column(self) -> ColumnList {
        ColumnList::one(select_item_for_meta(*self))
    }
}

impl<T> IntoColumns for Vec<Field<T>> {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.into_iter().map(select_item_for_field).collect(),
        }
    }
}

impl<T> IntoColumns for &[Field<T>] {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.iter().copied().map(select_item_for_field).collect(),
        }
    }
}

impl<T, const N: usize> IntoColumns for [Field<T>; N] {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.into_iter().map(select_item_for_field).collect(),
        }
    }
}

impl<T> IntoColumns for Vec<FieldRef<T>> {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.into_iter().map(select_item_for_ref).collect(),
        }
    }
}

impl<T> IntoColumns for &[FieldRef<T>] {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.iter().cloned().map(select_item_for_ref).collect(),
        }
    }
}

impl<T, const N: usize> IntoColumns for [FieldRef<T>; N] {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.into_iter().map(select_item_for_ref).collect(),
        }
    }
}

impl IntoColumns for Vec<Meta> {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.into_iter().map(select_item_for_meta).collect(),
        }
    }
}

impl IntoColumns for &[Meta] {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.iter().copied().map(select_item_for_meta).collect(),
        }
    }
}

impl<const N: usize> IntoColumns for [Meta; N] {
    fn into_columns(self) -> ColumnList {
        ColumnList {
            items: self.into_iter().map(select_item_for_meta).collect(),
        }
    }
}

macro_rules! impl_select_item_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> IntoColumns for ($($name,)+)
        where
            $($name: IntoColumn,)+
        {
            #[allow(non_snake_case)]
            fn into_columns(self) -> ColumnList {
                let ($($name,)+) = self;
                let mut items = Vec::new();
                $(items.extend($name.into_column().items);)+
                ColumnList { items }
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
