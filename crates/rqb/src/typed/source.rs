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

pub fn table(name: &'static str, fields: &'static [&'static Meta]) -> Source {
    Source::Table {
        name,
        alias: None,
        fields,
    }
}

pub fn view(name: &'static str, fields: &'static [&'static Meta]) -> Source {
    Source::View {
        name,
        alias: None,
        fields,
    }
}

pub fn cte(name: impl Into<String>, stmt: impl Into<Stmt>, fields: impl Into<Vec<Meta>>) -> Cte {
    Cte::new(name, stmt, fields)
}

pub fn cte_source(name: impl Into<String>, fields: impl Into<Vec<Meta>>) -> Source {
    Source::Cte {
        name: name.into(),
        alias: None,
        fields: fields.into(),
    }
}

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
    pub fn new(kind: JoinKind, source: Source, on: BoolExpr) -> Self {
        Self {
            kind,
            source,
            on: Some(on),
            lateral: false,
        }
    }

    pub fn lateral(kind: JoinKind, source: Source, on: BoolExpr) -> Self {
        Self {
            kind,
            source,
            on: Some(on),
            lateral: true,
        }
    }

    pub fn cross(source: Source) -> Self {
        Self {
            kind: JoinKind::Cross,
            source,
            on: None,
            lateral: false,
        }
    }

    pub fn cross_lateral(source: Source) -> Self {
        Self {
            kind: JoinKind::Cross,
            source,
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
            Self::Subquery { stmt, .. } => stmt.validate(),
            Self::Raw { sql, params, .. } => raw::validate_bind_count(sql, params.len()),
            Self::Function { args, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                Ok(())
            }
            Self::Table { .. } | Self::View { .. } | Self::Cte { .. } => Ok(()),
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
