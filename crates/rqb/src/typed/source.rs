use crate::typed::{Meta, Param, Stmt};
use crate::{Result, typed::raw};

#[derive(Clone, Debug)]
pub enum Source {
    Table {
        name: &'static str,
        fields: &'static [&'static Meta],
    },
    View {
        name: &'static str,
        fields: &'static [&'static Meta],
    },
    Cte {
        name: String,
        stmt: Box<Stmt>,
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
}

pub fn table(name: &'static str, fields: &'static [&'static Meta]) -> Source {
    Source::Table { name, fields }
}

pub fn view(name: &'static str, fields: &'static [&'static Meta]) -> Source {
    Source::View { name, fields }
}

impl Source {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Table { .. } => "table",
            Self::View { .. } => "view",
            Self::Cte { .. } => "cte",
            Self::Subquery { .. } => "subquery",
            Self::Raw { .. } => "raw",
        }
    }

    pub const fn is_table(&self) -> bool {
        matches!(self, Self::Table { .. })
    }

    pub(crate) fn for_each_field(&self, mut f: impl FnMut(&Meta)) {
        match self {
            Self::Table { fields, .. } | Self::View { fields, .. } => {
                for field in fields.iter().copied() {
                    f(field);
                }
            }
            Self::Cte { fields, .. } | Self::Subquery { fields, .. } | Self::Raw { fields, .. } => {
                for field in fields {
                    f(field);
                }
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Cte { stmt, .. } | Self::Subquery { stmt, .. } => stmt.validate(),
            Self::Raw { sql, params, .. } => raw::validate_bind_count(sql, params.len()),
            Self::Table { .. } | Self::View { .. } => Ok(()),
        }
    }

    pub(crate) fn collect_prefix_params(&self, params: &mut Vec<Param>) {
        if let Self::Cte { stmt, .. } = self {
            stmt.collect_params(params);
        }
    }

    pub(crate) fn collect_from_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Subquery { stmt, .. } => stmt.collect_params(params),
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
            Self::Table { .. } | Self::View { .. } | Self::Cte { .. } => {}
        }
    }
}
