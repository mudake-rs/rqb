use crate::typed::{BoolExpr, Meta, Param, Stmt};
use crate::{Result, typed::raw};

#[derive(Clone, Debug)]
pub enum Source {
    Table {
        name: &'static str,
        alias: Option<String>,
        fields: &'static [&'static Meta],
    },
    View {
        name: &'static str,
        alias: Option<String>,
        fields: &'static [&'static Meta],
    },
    Cte {
        name: String,
        alias: Option<String>,
        fields: Vec<Meta>,
    },
    Subquery {
        stmt: Box<Stmt>,
        alias: String,
        fields: Vec<Meta>,
    },
    Raw {
        sql: String,
        alias: String,
        params: Vec<Param>,
        fields: Vec<Meta>,
    },
    Function {
        name: &'static str,
        args: Vec<crate::typed::ValueExpr>,
        alias: String,
        fields: Vec<Meta>,
        ordinality: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CteMaterialization {
    Materialized,
    NotMaterialized,
}

impl CteMaterialization {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Materialized => "MATERIALIZED",
            Self::NotMaterialized => "NOT MATERIALIZED",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cte {
    pub name: String,
    pub columns: Vec<String>,
    pub recursive: bool,
    pub materialization: Option<CteMaterialization>,
    pub stmt: Box<Stmt>,
    pub fields: Vec<Meta>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Clone, Debug)]
pub struct Join {
    pub kind: JoinKind,
    pub source: Source,
    pub on: Option<BoolExpr>,
    pub lateral: bool,
}

/// Creates a table source from static metadata.
///
/// Generated schema modules usually expose this as `users::table()`.
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
pub fn cte(name: impl Into<String>, stmt: impl Into<Stmt>, fields: impl Into<Vec<Meta>>) -> Cte {
    Cte::new(name, stmt, fields)
}

/// Creates a source reference to a CTE defined elsewhere in the same query.
///
/// `Cte::source()` is usually clearer because it snapshots the CTE name and
/// field list from the CTE value.
pub fn cte_source(name: impl Into<String>, fields: impl Into<Vec<Meta>>) -> Source {
    Source::Cte {
        name: name.into(),
        alias: None,
        fields: fields.into(),
    }
}

/// Creates a subquery source with explicit exposed field metadata.
///
/// Prefer `Select::try_into_source(alias)` when the subquery projects plain
/// fields and rqb can infer metadata from the select list.
pub fn subquery(
    stmt: impl Into<Stmt>,
    alias: impl Into<String>,
    fields: impl Into<Vec<Meta>>,
) -> Source {
    Source::Subquery {
        stmt: Box::new(stmt.into()),
        alias: alias.into(),
        fields: fields.into(),
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
    fields: impl Into<Vec<Meta>>,
) -> Source {
    Source::Raw {
        sql: sql.into(),
        alias: alias.into(),
        params: params.into(),
        fields: fields.into(),
    }
}

/// Creates a table-valued function source with explicit exposed fields.
///
/// Use `Source::with_ordinality()` when the function should expose PostgreSQL
/// `WITH ORDINALITY`.
pub fn function_source(
    name: &'static str,
    args: impl Into<Vec<crate::typed::ValueExpr>>,
    alias: impl Into<String>,
    fields: impl Into<Vec<Meta>>,
) -> Source {
    Source::Function {
        name,
        args: args.into(),
        alias: alias.into(),
        fields: fields.into(),
        ordinality: false,
    }
}

impl JoinKind {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Inner => "JOIN",
            Self::Left => "LEFT JOIN",
            Self::Right => "RIGHT JOIN",
            Self::Full => "FULL JOIN",
            Self::Cross => "CROSS JOIN",
        }
    }

    pub const fn requires_condition(self) -> bool {
        !matches!(self, Self::Cross)
    }
}

impl Join {
    pub fn new(kind: JoinKind, source: impl Into<Source>, on: BoolExpr) -> Self {
        Self {
            kind,
            source: source.into(),
            on: Some(on),
            lateral: false,
        }
    }

    pub fn lateral(kind: JoinKind, source: impl Into<Source>, on: BoolExpr) -> Self {
        Self {
            kind,
            source: source.into(),
            on: Some(on),
            lateral: true,
        }
    }

    pub fn cross(source: impl Into<Source>) -> Self {
        Self {
            kind: JoinKind::Cross,
            source: source.into(),
            on: None,
            lateral: false,
        }
    }

    pub fn cross_lateral(source: impl Into<Source>) -> Self {
        Self {
            kind: JoinKind::Cross,
            source: source.into(),
            on: None,
            lateral: true,
        }
    }

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
    pub fn new(
        name: impl Into<String>,
        stmt: impl Into<Stmt>,
        fields: impl Into<Vec<Meta>>,
    ) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            recursive: false,
            materialization: None,
            stmt: Box::new(stmt.into()),
            fields: fields.into(),
        }
    }

    pub fn columns<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }

    pub fn materialized(mut self) -> Self {
        self.materialization = Some(CteMaterialization::Materialized);
        self
    }

    pub fn not_materialized(mut self) -> Self {
        self.materialization = Some(CteMaterialization::NotMaterialized);
        self
    }

    /// Returns a source reference for this CTE.
    ///
    /// The returned source owns a snapshot of the CTE name and exposed fields,
    /// so call this after finalizing the CTE metadata you want to query.
    pub fn source(&self) -> Source {
        cte_source(self.name.clone(), self.fields.clone())
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(crate::Error::InvalidCteShape {
                name: self.name.clone(),
                message: "CTE name cannot be empty",
            });
        }
        if let Some(count) = self.stmt.projection_count()
            && !self.fields.is_empty()
            && count != self.fields.len()
        {
            return Err(crate::Error::InvalidCteShape {
                name: self.name.clone(),
                message: "field count must match SELECT projection count",
            });
        }
        if !self.columns.is_empty() && self.columns.len() != self.fields.len() {
            return Err(crate::Error::InvalidCteShape {
                name: self.name.clone(),
                message: "column alias count must match exposed field count",
            });
        }
        self.stmt.validate()
    }

    pub(crate) fn collect_params(&self, params: &mut Vec<Param>) {
        self.stmt.collect_params(params);
    }
}

impl Source {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Table { .. } => "table",
            Self::View { .. } => "view",
            Self::Cte { .. } => "cte",
            Self::Subquery { .. } => "subquery",
            Self::Raw { .. } => "raw",
            Self::Function { .. } => "function",
        }
    }

    pub const fn is_table(&self) -> bool {
        matches!(self, Self::Table { .. })
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        let alias = alias.into();
        match &mut self {
            Self::Table { alias: current, .. }
            | Self::View { alias: current, .. }
            | Self::Cte { alias: current, .. } => *current = Some(alias),
            Self::Subquery { alias: current, .. }
            | Self::Raw { alias: current, .. }
            | Self::Function { alias: current, .. } => *current = alias,
        }
        self
    }

    pub fn with_ordinality(mut self) -> Self {
        if let Self::Function { ordinality, .. } = &mut self {
            *ordinality = true;
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
            | Self::Function { alias, .. } => Some(alias),
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
            | Self::Function { fields, .. } => fields.iter().for_each(f),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Subquery {
                stmt,
                alias,
                fields,
            } => {
                stmt.validate()?;
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
    use super::{CteMaterialization, Source, cte, cte_source, function_source, raw_source, table};
    use crate::typed::{Field, Meta, OpSet, Param, select};

    static ID_META: Meta = Meta::new("id", "id", "int4").ops(OpSet::ordered());
    static FIELDS: [&Meta; 1] = [&ID_META];
    const ID: Field<i32> = Field::new(&ID_META);

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
    fn cte_source_snapshots_name_and_fields() {
        let cte = cte(
            "ids",
            select(table("public.users", &FIELDS)).column(ID),
            vec![ID_META],
        );
        let source = cte.source().alias("i");

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
            crate::Error::InvalidCteShape { name, message }
                if name == "bad_ids" && message == "field count must match SELECT projection count"
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

        let function = function_source("generate_series", Vec::new(), "", vec![ID_META]);
        assert!(matches!(
            function.validate().unwrap_err(),
            crate::Error::InvalidSelectShape { message }
                if message == "function source alias cannot be empty"
        ));
    }

    #[test]
    fn function_source_validates_arguments_and_ordinality_is_opt_in() {
        let source = function_source(
            "generate_series",
            vec![
                crate::typed::ValueExpr::from(1_i32),
                crate::typed::ValueExpr::from(3_i32),
            ],
            "g",
            vec![ID_META],
        );
        assert!(matches!(
            &source,
            Source::Function {
                ordinality: false,
                ..
            }
        ));

        let with_ordinality = source.with_ordinality();
        assert!(matches!(
            with_ordinality,
            Source::Function {
                ordinality: true,
                ..
            }
        ));
    }

    #[test]
    fn cte_materialization_renders_known_postgres_tokens() {
        assert_eq!(CteMaterialization::Materialized.as_sql(), "MATERIALIZED");
        assert_eq!(
            CteMaterialization::NotMaterialized.as_sql(),
            "NOT MATERIALIZED"
        );

        let source = cte_source("ids", vec![ID_META]);
        assert_eq!(source.kind(), "cte");
    }
}
