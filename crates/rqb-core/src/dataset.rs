use crate::expr::Expr;
use crate::field::{Field, FieldRef};
use crate::raw::RawSql;
use crate::request::SelectQuery;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Table {
        schema: Option<String>,
        name: String,
        alias: Option<String>,
    },
    View {
        schema: Option<String>,
        name: String,
        alias: Option<String>,
    },
    Cte {
        name: String,
        alias: Option<String>,
    },
    Raw {
        sql: String,
        alias: String,
    },
}

impl Source {
    pub fn table(name: impl Into<String>) -> Self {
        Self::Table {
            schema: None,
            name: name.into(),
            alias: None,
        }
    }

    pub fn view(name: impl Into<String>) -> Self {
        Self::View {
            schema: None,
            name: name.into(),
            alias: None,
        }
    }

    pub fn cte(name: impl Into<String>) -> Self {
        Self::Cte {
            name: name.into(),
            alias: None,
        }
    }

    pub fn raw(sql: impl Into<String>, alias: impl Into<String>) -> Self {
        Self::Raw {
            sql: sql.into(),
            alias: alias.into(),
        }
    }

    pub fn schema(mut self, schema: impl Into<String>) -> Self {
        match &mut self {
            Self::Table { schema: s, .. } | Self::View { schema: s, .. } => {
                *s = Some(schema.into())
            }
            Self::Cte { .. } | Self::Raw { .. } => {}
        }
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        match &mut self {
            Self::Table { alias: a, .. }
            | Self::View { alias: a, .. }
            | Self::Cte { alias: a, .. } => *a = Some(alias.into()),
            Self::Raw { alias: a, .. } => *a = alias.into(),
        }
        self
    }

    pub fn alias_name(&self) -> Option<&str> {
        match self {
            Self::Table { alias, .. } | Self::View { alias, .. } | Self::Cte { alias, .. } => {
                alias.as_deref()
            }
            Self::Raw { alias, .. } => Some(alias),
        }
    }

    pub fn base_name(&self) -> &str {
        match self {
            Self::Table { name, .. } | Self::View { name, .. } | Self::Cte { name, .. } => name,
            Self::Raw { alias, .. } => alias,
        }
    }

    pub fn sql_qualifier(&self) -> &str {
        self.alias_name().unwrap_or_else(|| self.base_name())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dataset {
    pub api_name: String,
    pub source: Source,
    pub fields: Vec<Field>,
    pub default_limit: u32,
    pub max_limit: u32,
}

impl Dataset {
    pub fn table(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(name.clone(), Source::table(name))
    }

    pub fn view(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(name.clone(), Source::view(name))
    }

    pub fn raw(sql: impl Into<String>, alias: impl Into<String>) -> Self {
        let alias = alias.into();
        Self::new(alias.clone(), Source::raw(sql, alias))
    }

    pub fn cte(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(name.clone(), Source::cte(name))
    }

    pub fn new(api_name: impl Into<String>, source: Source) -> Self {
        Self {
            api_name: api_name.into(),
            source,
            fields: Vec::new(),
            default_limit: 100,
            max_limit: 1000,
        }
    }

    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    pub fn fields<I>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = Field>,
    {
        self.fields.extend(fields);
        self
    }

    pub fn default_limit(mut self, default_limit: u32) -> Self {
        self.default_limit = default_limit;
        self
    }

    pub fn max_limit(mut self, max_limit: u32) -> Self {
        self.max_limit = max_limit;
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.source = self.source.alias(alias);
        self
    }

    pub fn source_alias(&self) -> Option<&str> {
        self.source.alias_name()
    }

    pub fn source_name(&self) -> &str {
        self.source.base_name()
    }

    pub fn sql_qualifier(&self) -> &str {
        self.source.sql_qualifier()
    }

    pub fn matches_qualifier(&self, qualifier: &str) -> bool {
        self.api_name == qualifier
            || self.source_name() == qualifier
            || self.source_alias() == Some(qualifier)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    dataset: Dataset,
}

impl Relation {
    pub fn new(dataset: impl Into<Dataset>) -> Self {
        Self {
            dataset: dataset.into(),
        }
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.dataset = self.dataset.alias(alias);
        self
    }

    pub fn dataset(&self) -> &Dataset {
        &self.dataset
    }

    pub fn field(&self, field: Field) -> FieldRef {
        match self.dataset.source_alias() {
            Some(qualifier) => field.on(qualifier),
            None => field.into(),
        }
    }
}

impl From<Relation> for Dataset {
    fn from(value: Relation) -> Self {
        value.dataset
    }
}

impl From<&Relation> for Dataset {
    fn from(value: &Relation) -> Self {
        value.dataset.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl JoinKind {
    pub fn as_sql(self) -> &'static str {
        match self {
            Self::Inner => "JOIN",
            Self::Left => "LEFT JOIN",
            Self::Right => "RIGHT JOIN",
            Self::Full => "FULL JOIN",
            Self::Cross => "CROSS JOIN",
        }
    }

    pub fn requires_condition(self) -> bool {
        !matches!(self, Self::Cross)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    pub kind: JoinKind,
    pub dataset: Dataset,
    pub on: Option<Expr>,
}

impl Join {
    pub fn new(kind: JoinKind, dataset: impl Into<Dataset>, on: impl Into<Expr>) -> Self {
        Self {
            kind,
            dataset: dataset.into(),
            on: Some(on.into()),
        }
    }

    pub fn cross(dataset: impl Into<Dataset>) -> Self {
        Self {
            kind: JoinKind::Cross,
            dataset: dataset.into(),
            on: None,
        }
    }
}

impl From<Source> for Dataset {
    fn from(source: Source) -> Self {
        let api_name = match &source {
            Source::Table { name, .. } | Source::View { name, .. } | Source::Cte { name, .. } => {
                name.clone()
            }
            Source::Raw { alias, .. } => alias.clone(),
        };
        Self::new(api_name, source)
    }
}

impl From<&Dataset> for Dataset {
    fn from(value: &Dataset) -> Self {
        value.clone()
    }
}

impl From<&str> for Dataset {
    fn from(value: &str) -> Self {
        Self::table(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cte {
    pub name: String,
    pub columns: Vec<String>,
    pub recursive: bool,
    pub body: CteBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CteBody {
    Raw(RawSql),
    Select(Box<SelectQuery>),
}

pub fn cte(name: impl Into<String>, body: impl Into<CteBody>) -> Cte {
    Cte::new(name, body)
}

impl Cte {
    pub fn new(name: impl Into<String>, body: impl Into<CteBody>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            recursive: false,
            body: body.into(),
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
}

impl From<RawSql> for CteBody {
    fn from(value: RawSql) -> Self {
        Self::Raw(value)
    }
}

impl From<SelectQuery> for CteBody {
    fn from(value: SelectQuery) -> Self {
        Self::Select(Box::new(value))
    }
}

#[cfg(test)]
mod tests {
    use crate::field::FieldType;

    use super::*;

    #[test]
    fn relation_qualifies_fields_only_when_aliased() {
        let id = Field::new("id", FieldType::Uuid);

        let root = Relation::new(Dataset::table("users").field(id));
        assert_eq!(root.field(id).qualifier(), None);

        let user = root.alias("u");
        assert_eq!(user.field(id).qualifier(), Some("u"));
        assert_eq!(user.dataset().source_alias(), Some("u"));
    }
}
