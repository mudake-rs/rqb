use crate::{BoolExpr, Field, FieldRef, IntoRowValues, Meta, Param, Stmt, ValueExpr};
use crate::{Result, raw};

/// Relation-like object that can appear in a `FROM`, `JOIN`, `UPDATE`, or write target.
#[derive(Clone, Debug)]
#[must_use]
#[non_exhaustive]
pub enum Source {
    /// Database table from static schema metadata.
    Table {
        /// Qualified table name, for example `public.users`.
        name: &'static str,
        /// Optional SQL alias.
        alias: Option<String>,
        /// Exposed fields.
        fields: &'static [&'static Meta],
    },
    /// Database view from static schema metadata.
    View {
        /// Qualified view name.
        name: &'static str,
        /// Optional SQL alias.
        alias: Option<String>,
        /// Exposed fields.
        fields: &'static [&'static Meta],
    },
    /// Reference to a CTE defined in the surrounding statement.
    Cte {
        /// CTE name.
        name: String,
        /// Optional SQL alias.
        alias: Option<String>,
        /// Exposed fields.
        fields: Vec<Meta>,
    },
    /// Derived table from a nested query.
    Subquery {
        /// Query rendered inside parentheses.
        stmt: Box<Stmt>,
        /// Required SQL alias.
        alias: String,
        /// Exposed fields.
        fields: Vec<Meta>,
    },
    /// Server-owned raw SQL source.
    Raw {
        /// Raw SQL fragment using rqb `?` placeholders.
        sql: String,
        /// Required SQL alias.
        alias: String,
        /// Bind parameters for the raw fragment.
        params: Vec<Param>,
        /// Exposed fields.
        fields: Vec<Meta>,
    },
    /// Table-valued function source.
    Function {
        /// Function name rendered as SQL.
        name: &'static str,
        /// Function arguments.
        args: Vec<ValueExpr>,
        /// Required SQL alias.
        alias: String,
        /// Exposed fields.
        fields: Vec<Meta>,
        /// Whether to render `WITH ORDINALITY`.
        ordinality: bool,
    },
    /// Inline `VALUES` table source.
    Values {
        /// Row values.
        rows: Vec<Vec<ValueExpr>>,
        /// Required SQL alias.
        alias: String,
        /// Exposed fields.
        fields: Vec<Meta>,
    },
}

/// Table-valued function source before it is converted into a general [`Source`].
///
/// This wrapper keeps `WITH ORDINALITY` available only for sources where
/// PostgreSQL supports it.
#[derive(Clone, Debug)]
#[must_use]
#[non_exhaustive]
pub struct FunctionSource {
    /// Function name rendered as SQL.
    pub name: &'static str,
    /// Function arguments.
    pub args: Vec<ValueExpr>,
    /// Required SQL alias.
    pub alias: String,
    /// Exposed fields.
    pub fields: Vec<Meta>,
    /// Whether to render `WITH ORDINALITY`.
    pub ordinality: bool,
}

/// PostgreSQL CTE materialization hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CteMaterialization {
    /// Render `MATERIALIZED`.
    Materialized,
    /// Render `NOT MATERIALIZED`.
    NotMaterialized,
}

impl CteMaterialization {
    /// Returns the SQL keyword for this materialization hint.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Materialized => "MATERIALIZED",
            Self::NotMaterialized => "NOT MATERIALIZED",
        }
    }
}

/// Common table expression definition.
#[derive(Clone, Debug)]
#[must_use]
#[non_exhaustive]
pub struct Cte {
    /// CTE name.
    pub name: String,
    /// Optional explicit column aliases.
    pub columns: Vec<String>,
    /// Whether to render `WITH RECURSIVE`.
    pub recursive: bool,
    /// Optional materialization hint.
    pub materialization: Option<CteMaterialization>,
    /// CTE body statement.
    pub stmt: Box<Stmt>,
    /// Exposed field metadata for querying this CTE.
    pub fields: Vec<Meta>,
}

/// Converts fields, metadata, and tuples of either into exposed metadata.
///
/// This keeps CTE/subquery/raw-source calls from repeating `*field.meta` when
/// the field already carries the metadata rqb needs.
pub trait IntoFieldMetas {
    /// Converts this value into exposed field metadata.
    fn into_field_metas(self) -> Vec<Meta>;
}

/// SQL join kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    /// Inner join.
    Inner,
    /// Left outer join.
    Left,
    /// Right outer join.
    Right,
    /// Full outer join.
    Full,
    /// Cross join.
    Cross,
}

/// Join clause attached to a select statement.
#[derive(Clone, Debug)]
#[must_use]
#[non_exhaustive]
pub struct Join {
    /// Join kind.
    pub kind: JoinKind,
    /// Joined source.
    pub source: Source,
    /// Join condition. Cross joins do not require one.
    pub on: Option<BoolExpr>,
    /// Whether to render `LATERAL`.
    pub lateral: bool,
}

/// Creates a table source from static metadata.
///
/// Generated schema modules usually expose this as `users::table()`.
#[inline]
pub fn table(name: &'static str, fields: &'static [&'static Meta]) -> Source {
    Source::Table {
        name,
        alias: None,
        fields,
    }
}

/// Creates a view source from static metadata.
///
/// Generated schema modules usually expose this as `search_view::view()`.
#[inline]
pub fn view(name: &'static str, fields: &'static [&'static Meta]) -> Source {
    Source::View {
        name,
        alias: None,
        fields,
    }
}

/// Creates a CTE with explicit exposed field metadata.
///
/// Prefer `Select::try_into_cte(name)` when the CTE projects plain fields and
/// rqb can infer the exposed metadata from the select list.
pub fn cte(name: impl Into<String>, stmt: impl Into<Stmt>, fields: impl IntoFieldMetas) -> Cte {
    Cte::new(name, stmt, fields)
}

/// Creates a source reference to a CTE defined elsewhere in the same query.
///
/// `Cte::source()` is usually clearer because it snapshots the CTE name and
/// field list from the CTE value.
pub fn cte_ref(name: impl Into<String>, fields: impl IntoFieldMetas) -> Source {
    Source::Cte {
        name: name.into(),
        alias: None,
        fields: fields.into_field_metas(),
    }
}

/// Creates a subquery source with explicit exposed field metadata.
///
/// Prefer `Select::try_into_source(alias)` when the subquery projects plain
/// fields and rqb can infer metadata from the select list.
pub fn subquery(
    stmt: impl Into<Stmt>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> Source {
    Source::Subquery {
        stmt: Box::new(stmt.into()),
        alias: alias.into(),
        fields: fields.into_field_metas(),
    }
}

/// Creates a raw SQL source with explicit exposed field metadata.
///
/// Raw source SQL is server-owned. rqb validates bind counts and renders the
/// exposed field list as the derived-table column list.
pub fn raw_source(
    sql: impl Into<String>,
    alias: impl Into<String>,
    params: impl Into<Vec<Param>>,
    fields: impl IntoFieldMetas,
) -> Source {
    Source::Raw {
        sql: sql.into(),
        alias: alias.into(),
        params: params.into(),
        fields: fields.into_field_metas(),
    }
}

/// Creates a table-valued function source with explicit exposed fields.
pub fn function_source(
    name: &'static str,
    args: impl Into<Vec<ValueExpr>>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    FunctionSource {
        name,
        args: args.into(),
        alias: alias.into(),
        fields: fields.into_field_metas(),
        ordinality: false,
    }
}

/// Creates a `generate_series(start, stop)` table-valued function source.
pub fn generate_series_source(
    start: impl Into<ValueExpr>,
    stop: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source(
        "generate_series",
        vec![start.into(), stop.into()],
        alias,
        fields,
    )
}

/// Creates a `generate_series(start, stop, step)` table-valued function source.
pub fn generate_series_step_source(
    start: impl Into<ValueExpr>,
    stop: impl Into<ValueExpr>,
    step: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source(
        "generate_series",
        vec![start.into(), stop.into(), step.into()],
        alias,
        fields,
    )
}

/// Creates an `unnest(array)` table-valued function source.
pub fn unnest_source(
    array: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("unnest", vec![array.into()], alias, fields)
}

/// Creates a `generate_subscripts(array, dim)` table-valued function source.
pub fn generate_subscripts_source(
    array: impl Into<ValueExpr>,
    dim: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source(
        "generate_subscripts",
        vec![array.into(), dim.into()],
        alias,
        fields,
    )
}

/// Creates a `regexp_split_to_table(text, pattern)` source.
pub fn regexp_split_to_table_source(
    text: impl Into<ValueExpr>,
    pattern: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source(
        "regexp_split_to_table",
        vec![text.into(), pattern.into()],
        alias,
        fields,
    )
}

/// Creates a `json_object_keys(json)` source.
pub fn json_object_keys_source(
    value: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("json_object_keys", vec![value.into()], alias, fields)
}

/// Creates a `jsonb_object_keys(jsonb)` source.
pub fn jsonb_object_keys_source(
    value: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("jsonb_object_keys", vec![value.into()], alias, fields)
}

/// Creates a `json_each(json)` source.
pub fn json_each_source(
    value: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("json_each", vec![value.into()], alias, fields)
}

/// Creates a `jsonb_each(jsonb)` source.
pub fn jsonb_each_source(
    value: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("jsonb_each", vec![value.into()], alias, fields)
}

/// Creates a `json_array_elements(json)` source.
pub fn json_array_elements_source(
    value: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("json_array_elements", vec![value.into()], alias, fields)
}

/// Creates a `jsonb_array_elements(jsonb)` source.
pub fn jsonb_array_elements_source(
    value: impl Into<ValueExpr>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> FunctionSource {
    function_source("jsonb_array_elements", vec![value.into()], alias, fields)
}

/// Creates a `FROM (VALUES ...) AS alias(columns...)` source.
pub fn values_source<R>(
    rows: impl IntoIterator<Item = R>,
    alias: impl Into<String>,
    fields: impl IntoFieldMetas,
) -> Source
where
    R: IntoRowValues,
{
    Source::Values {
        rows: rows
            .into_iter()
            .map(IntoRowValues::into_row_values)
            .collect(),
        alias: alias.into(),
        fields: fields.into_field_metas(),
    }
}

impl IntoFieldMetas for () {
    fn into_field_metas(self) -> Vec<Meta> {
        Vec::new()
    }
}

impl IntoFieldMetas for Meta {
    fn into_field_metas(self) -> Vec<Meta> {
        vec![self]
    }
}

impl IntoFieldMetas for &Meta {
    fn into_field_metas(self) -> Vec<Meta> {
        vec![*self]
    }
}

impl<T> IntoFieldMetas for Field<T> {
    fn into_field_metas(self) -> Vec<Meta> {
        vec![*self.meta]
    }
}

impl<T> IntoFieldMetas for &Field<T> {
    fn into_field_metas(self) -> Vec<Meta> {
        vec![*self.meta]
    }
}

impl<T> IntoFieldMetas for FieldRef<T> {
    fn into_field_metas(self) -> Vec<Meta> {
        vec![*self.meta]
    }
}

impl<T> IntoFieldMetas for &FieldRef<T> {
    fn into_field_metas(self) -> Vec<Meta> {
        vec![*self.meta]
    }
}

impl IntoFieldMetas for Vec<Meta> {
    fn into_field_metas(self) -> Vec<Meta> {
        self
    }
}

impl IntoFieldMetas for &[Meta] {
    fn into_field_metas(self) -> Vec<Meta> {
        self.to_vec()
    }
}

impl<const N: usize> IntoFieldMetas for [Meta; N] {
    fn into_field_metas(self) -> Vec<Meta> {
        self.into_iter().collect()
    }
}

impl<const N: usize> IntoFieldMetas for [&Meta; N] {
    fn into_field_metas(self) -> Vec<Meta> {
        self.into_iter().copied().collect()
    }
}

macro_rules! impl_field_meta_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name),+> IntoFieldMetas for ($($name,)+)
        where
            $($name: IntoFieldMetas,)+
        {
            #[allow(non_snake_case)]
            fn into_field_metas(self) -> Vec<Meta> {
                let ($($name,)+) = self;
                let mut fields = Vec::new();
                $(fields.extend($name.into_field_metas());)+
                fields
            }
        }
    };
}

impl_field_meta_tuple!(A, B);
impl_field_meta_tuple!(A, B, C);
impl_field_meta_tuple!(A, B, C, D);
impl_field_meta_tuple!(A, B, C, D, E);
impl_field_meta_tuple!(A, B, C, D, E, F);
impl_field_meta_tuple!(A, B, C, D, E, F, G);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_field_meta_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

impl JoinKind {
    /// Returns the SQL keyword for this join kind.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Inner => "JOIN",
            Self::Left => "LEFT JOIN",
            Self::Right => "RIGHT JOIN",
            Self::Full => "FULL JOIN",
            Self::Cross => "CROSS JOIN",
        }
    }

    /// Returns true when this join kind requires an `ON` condition.
    #[inline]
    pub const fn requires_condition(self) -> bool {
        !matches!(self, Self::Cross)
    }
}

impl Join {
    /// Creates a non-lateral join with an `ON` condition.
    pub fn new(kind: JoinKind, source: impl Into<Source>, on: BoolExpr) -> Self {
        Self {
            kind,
            source: source.into(),
            on: Some(on),
            lateral: false,
        }
    }

    /// Creates a lateral join with an `ON` condition.
    pub fn lateral(kind: JoinKind, source: impl Into<Source>, on: BoolExpr) -> Self {
        Self {
            kind,
            source: source.into(),
            on: Some(on),
            lateral: true,
        }
    }

    /// Creates a cross join.
    pub fn cross(source: impl Into<Source>) -> Self {
        Self {
            kind: JoinKind::Cross,
            source: source.into(),
            on: None,
            lateral: false,
        }
    }

    /// Creates a lateral cross join.
    pub fn cross_lateral(source: impl Into<Source>) -> Self {
        Self {
            kind: JoinKind::Cross,
            source: source.into(),
            on: None,
            lateral: true,
        }
    }

    /// Validates the joined source and required join condition.
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        match (&self.on, self.kind.requires_condition()) {
            (Some(on), _) => on.validate(),
            (None, false) => Ok(()),
            (None, true) => Err(crate::Error::MissingJoinCondition {
                join: self.kind.as_sql(),
            }),
        }
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        self.source.collect_from_params(params);
        if let Some(on) = &self.on {
            on.collect_params(params);
        }
    }
}

impl Cte {
    /// Creates a CTE with explicit exposed field metadata.
    pub fn new(
        name: impl Into<String>,
        stmt: impl Into<Stmt>,
        fields: impl IntoFieldMetas,
    ) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            recursive: false,
            materialization: None,
            stmt: Box::new(stmt.into()),
            fields: fields.into_field_metas(),
        }
    }

    /// Sets explicit CTE column aliases.
    pub fn columns<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    /// Marks the CTE and surrounding `WITH` clause as recursive.
    #[inline]
    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }

    /// Adds the PostgreSQL `MATERIALIZED` hint.
    #[inline]
    pub fn materialized(mut self) -> Self {
        self.materialization = Some(CteMaterialization::Materialized);
        self
    }

    /// Adds the PostgreSQL `NOT MATERIALIZED` hint.
    #[inline]
    pub fn not_materialized(mut self) -> Self {
        self.materialization = Some(CteMaterialization::NotMaterialized);
        self
    }

    /// Returns a source reference for this CTE.
    ///
    /// The returned source owns a snapshot of the CTE name and exposed fields,
    /// so call this after finalizing the CTE metadata you want to query.
    #[inline]
    pub fn source(&self) -> Source {
        cte_ref(self.name.clone(), self.fields.clone())
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(crate::Error::invalid_cte_shape(
                self.name.clone(),
                "CTE name cannot be empty",
            ));
        }
        if let Some(count) = self.stmt.projection_count()
            && !self.fields.is_empty()
            && count != self.fields.len()
        {
            return Err(crate::Error::invalid_cte_shape(
                self.name.clone(),
                "field count must match SELECT projection count",
            ));
        }
        if !self.columns.is_empty() && self.columns.len() != self.fields.len() {
            return Err(crate::Error::invalid_cte_shape(
                self.name.clone(),
                "column alias count must match exposed field count",
            ));
        }
        self.stmt.validate()
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        self.stmt.collect_params(params);
    }
}

impl From<&Cte> for Source {
    fn from(cte: &Cte) -> Self {
        cte.source()
    }
}

impl FunctionSource {
    /// Enables PostgreSQL `WITH ORDINALITY`.
    ///
    /// The ordinality column remains explicit metadata: include it in the
    /// `fields` argument when callers need to project it through rqb.
    #[inline]
    pub fn with_ordinality(mut self) -> Self {
        self.ordinality = true;
        self
    }
}

impl From<FunctionSource> for Source {
    fn from(source: FunctionSource) -> Self {
        Self::Function {
            name: source.name,
            args: source.args,
            alias: source.alias,
            fields: source.fields,
            ordinality: source.ordinality,
        }
    }
}

impl Source {
    /// Returns a stable source-kind name for diagnostics.
    #[inline]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Table { .. } => "table",
            Self::View { .. } => "view",
            Self::Cte { .. } => "cte",
            Self::Subquery { .. } => "subquery",
            Self::Raw { .. } => "raw",
            Self::Function { .. } => "function",
            Self::Values { .. } => "values",
        }
    }

    /// Returns true when this source is a table.
    #[inline]
    pub const fn is_table(&self) -> bool {
        matches!(self, Self::Table { .. })
    }

    /// Sets or replaces the SQL alias for this source.
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        let alias = alias.into();
        match &mut self {
            Self::Table { alias: current, .. }
            | Self::View { alias: current, .. }
            | Self::Cte { alias: current, .. } => *current = Some(alias),
            Self::Subquery { alias: current, .. }
            | Self::Raw { alias: current, .. }
            | Self::Function { alias: current, .. }
            | Self::Values { alias: current, .. } => *current = alias,
        }
        self
    }

    pub(crate) fn explicit_alias(&self) -> Option<&str> {
        match self {
            Self::Table { alias, .. } | Self::View { alias, .. } | Self::Cte { alias, .. } => {
                alias.as_deref()
            }
            Self::Subquery { alias, .. }
            | Self::Raw { alias, .. }
            | Self::Function { alias, .. }
            | Self::Values { alias, .. } => Some(alias),
        }
    }

    pub(crate) fn for_each_field(&self, mut f: impl FnMut(&Meta)) {
        match self {
            Self::Table { fields, .. } | Self::View { fields, .. } => {
                for field in fields.iter().copied() {
                    f(field);
                }
            }
            Self::Cte { fields, .. }
            | Self::Subquery { fields, .. }
            | Self::Raw { fields, .. }
            | Self::Function { fields, .. }
            | Self::Values { fields, .. } => fields.iter().for_each(f),
        }
    }

    /// Validates this source before rendering.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Subquery {
                stmt,
                alias,
                fields,
            } => {
                stmt.validate_query_statement(
                    "subquery source must be SELECT, set, or raw statement",
                )?;
                if let Some(count) = stmt.projection_count()
                    && !fields.is_empty()
                    && count != fields.len()
                {
                    return Err(crate::Error::InvalidSelectShape {
                        message: "subquery field count must match SELECT projection count",
                    });
                }
                if alias.is_empty() {
                    return Err(crate::Error::InvalidSelectShape {
                        message: "subquery alias cannot be empty",
                    });
                }
                Ok(())
            }
            Self::Raw {
                sql, alias, params, ..
            } => {
                validate_source_alias(alias, "raw source alias cannot be empty")?;
                raw::validate_bind_count(sql, params.len())
            }
            Self::Function { args, alias, .. } => {
                validate_source_alias(alias, "function source alias cannot be empty")?;
                for arg in args {
                    arg.validate()?;
                }
                Ok(())
            }
            Self::Values {
                rows,
                alias,
                fields,
            } => {
                validate_source_alias(alias, "values source alias cannot be empty")?;
                if rows.is_empty() {
                    return Err(crate::Error::InvalidSelectShape {
                        message: "values source requires at least one row",
                    });
                }
                if fields.is_empty() {
                    return Err(crate::Error::InvalidSelectShape {
                        message: "values source fields cannot be empty",
                    });
                }
                for row in rows {
                    if row.len() != fields.len() {
                        return Err(crate::Error::InvalidSelectShape {
                            message: "values source row arity must match exposed field count",
                        });
                    }
                    for value in row {
                        value.validate()?;
                    }
                }
                Ok(())
            }
            Self::Cte { name, .. } => {
                if name.is_empty() {
                    return Err(crate::Error::InvalidSelectShape {
                        message: "CTE source name cannot be empty",
                    });
                }
                Ok(())
            }
            Self::Table { .. } | Self::View { .. } => Ok(()),
        }
    }

    pub(crate) fn collect_from_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Subquery { stmt, .. } => stmt.collect_params(params),
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
            Self::Function { args, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
            }
            Self::Values { rows, .. } => {
                for row in rows {
                    for value in row {
                        value.collect_params(params);
                    }
                }
            }
            Self::Table { .. } | Self::View { .. } | Self::Cte { .. } => {}
        }
    }
}

fn validate_source_alias(alias: &str, message: &'static str) -> Result<()> {
    if alias.is_empty() {
        return Err(crate::Error::InvalidSelectShape { message });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CteMaterialization, IntoFieldMetas, Source, cte, cte_ref, function_source,
        generate_series_step_source, raw_source, table, values_source,
    };
    use crate::{Field, Meta, OpSet, Param, select};

    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<i32> = Field::new(&ID_META);

    #[test]
    fn into_field_metas_accepts_fields_metadata_and_tuples() {
        assert_eq!(().into_field_metas(), Vec::<Meta>::new());
        assert_eq!(ID.into_field_metas(), vec![ID_META]);
        assert_eq!((&ID_META).into_field_metas(), vec![ID_META]);
        assert_eq!((ID, &ID_META).into_field_metas(), vec![ID_META, ID_META]);
        assert_eq!(
            (
                ID, ID, ID, ID, ID, ID, ID, ID, ID, ID, ID, ID, ID, ID, ID, ID,
            )
                .into_field_metas()
                .len(),
            16
        );
    }

    #[test]
    fn table_and_view_aliases_are_optional_but_raw_sources_are_always_aliased() {
        let table = table("public.users", &FIELDS);
        assert_eq!(table.explicit_alias(), None);
        assert!(
            matches!(table.alias("u"), Source::Table { alias: Some(alias), .. } if alias == "u")
        );

        let raw = raw_source(
            "select ? as id",
            "generated",
            [Param::typed(1_i32)],
            vec![ID_META],
        );
        assert_eq!(raw.explicit_alias(), Some("generated"));
    }

    #[test]
    fn cte_ref_snapshots_name_and_fields() {
        let cte = cte(
            "ids",
            select(table("public.users", &FIELDS)).column(ID),
            vec![ID_META],
        );
        let source = Source::from(&cte).alias("i");

        assert!(matches!(
            source,
            Source::Cte {
                name,
                alias: Some(alias),
                fields,
            } if name == "ids" && alias == "i" && fields == vec![ID_META]
        ));
    }

    #[test]
    fn cte_validates_field_count_against_select_projection() {
        let cte = cte(
            "bad_ids",
            select(table("public.users", &FIELDS)).column(ID),
            vec![ID_META, ID_META],
        );

        assert!(matches!(
            cte.validate().unwrap_err(),
            crate::Error::InvalidCteShape(err) if err.name == "bad_ids" && err.message == "field count must match SELECT projection count"
        ));
    }

    #[test]
    fn subquery_validates_field_count_against_select_projection() {
        let source = super::subquery(
            select(table("public.users", &FIELDS)).column(ID),
            "ids",
            vec![ID_META, ID_META],
        );

        assert!(matches!(
            source.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "subquery field count must match SELECT projection count"
        ));
    }

    #[test]
    fn raw_source_validates_bind_count() {
        let source = raw_source(
            "select ? as id",
            "generated",
            Vec::<Param>::new(),
            vec![ID_META],
        );

        assert!(matches!(
            source.validate().unwrap_err(),
            crate::Error::RawBindMismatch {
                placeholders: 1,
                binds: 0
            }
        ));
    }

    #[test]
    fn raw_and_function_sources_require_aliases() {
        let raw = raw_source("select 1 as id", "", Vec::<Param>::new(), vec![ID_META]);
        assert!(matches!(
            raw.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "raw source alias cannot be empty"
        ));

        let function: Source =
            function_source("generate_series", Vec::new(), "", vec![ID_META]).into();
        assert!(matches!(
            function.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "function source alias cannot be empty"
        ));

        let values = values_source([[1_i32]], "", ID);
        assert!(matches!(
            values.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "values source alias cannot be empty"
        ));
    }

    #[test]
    fn function_source_validates_arguments_and_ordinality_is_opt_in() {
        let source = generate_series_step_source(1_i32, 3_i32, 1_i32, "g", ID);
        assert!(!source.ordinality);

        let with_ordinality = source.with_ordinality();
        assert!(with_ordinality.ordinality);

        let source: Source = with_ordinality.into();
        assert!(matches!(
            source,
            Source::Function {
                ordinality: true,
                ..
            }
        ));
    }

    #[test]
    fn values_source_validates_rows_and_exposed_fields() {
        let valid = values_source([[1_i32], [2_i32]], "input", ID);
        valid.validate().unwrap();

        let empty_rows = values_source(Vec::<[i32; 1]>::new(), "input", ID);
        assert!(matches!(
            empty_rows.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "values source requires at least one row"
        ));

        let empty_fields = values_source([[1_i32]], "input", ());
        assert!(matches!(
            empty_fields.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "values source fields cannot be empty"
        ));

        let wrong_arity = values_source([(1_i32, 2_i32)], "input", ID);
        assert!(matches!(
            wrong_arity.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "values source row arity must match exposed field count"
        ));
    }

    #[test]
    fn cte_materialization_renders_known_postgres_tokens() {
        assert_eq!(CteMaterialization::Materialized.as_sql(), "MATERIALIZED");
        assert_eq!(
            CteMaterialization::NotMaterialized.as_sql(),
            "NOT MATERIALIZED"
        );

        let source = cte_ref("ids", vec![ID_META]);
        assert_eq!(source.kind(), "cte");
    }
}
