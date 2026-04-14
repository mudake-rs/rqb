use std::borrow::Cow;

use crate::builder::SelectBuilder;
use crate::expr::Expr;
use crate::field::{Field, FieldRef};
use crate::query::{QueryExpr, SetQuery};
use crate::raw::RawSql;
use crate::request::SelectQuery;

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Table {
        schema: Option<Cow<'static, str>>,
        name: Cow<'static, str>,
        alias: Option<Cow<'static, str>>,
    },
    View {
        schema: Option<Cow<'static, str>>,
        name: Cow<'static, str>,
        alias: Option<Cow<'static, str>>,
    },
    Cte {
        name: Cow<'static, str>,
        alias: Option<Cow<'static, str>>,
    },
    Raw {
        sql: String,
        alias: Cow<'static, str>,
    },
    Subquery {
        query: Box<QueryExpr>,
        alias: Cow<'static, str>,
    },
}

impl Source {
    pub fn table(name: impl Into<String>) -> Self {
        Self::Table {
            schema: None,
            name: Cow::Owned(name.into()),
            alias: None,
        }
    }

    pub fn static_table(name: &'static str) -> Self {
        Self::Table {
            schema: None,
            name: Cow::Borrowed(name),
            alias: None,
        }
    }

    pub fn view(name: impl Into<String>) -> Self {
        Self::View {
            schema: None,
            name: Cow::Owned(name.into()),
            alias: None,
        }
    }

    pub fn static_view(name: &'static str) -> Self {
        Self::View {
            schema: None,
            name: Cow::Borrowed(name),
            alias: None,
        }
    }

    pub fn cte(name: impl Into<String>) -> Self {
        Self::Cte {
            name: Cow::Owned(name.into()),
            alias: None,
        }
    }

    pub fn raw(sql: impl Into<String>, alias: impl Into<String>) -> Self {
        Self::Raw {
            sql: sql.into(),
            alias: Cow::Owned(alias.into()),
        }
    }

    pub fn subquery(query: impl Into<QueryExpr>, alias: impl Into<String>) -> Self {
        Self::Subquery {
            query: Box::new(query.into()),
            alias: Cow::Owned(alias.into()),
        }
    }

    pub fn schema(mut self, schema: impl Into<String>) -> Self {
        match &mut self {
            Self::Table { schema: s, .. } | Self::View { schema: s, .. } => {
                *s = Some(Cow::Owned(schema.into()))
            }
            Self::Cte { .. } | Self::Raw { .. } | Self::Subquery { .. } => {}
        }
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        match &mut self {
            Self::Table { alias: a, .. }
            | Self::View { alias: a, .. }
            | Self::Cte { alias: a, .. } => *a = Some(Cow::Owned(alias.into())),
            Self::Raw { alias: a, .. } | Self::Subquery { alias: a, .. } => {
                *a = Cow::Owned(alias.into())
            }
        }
        self
    }

    pub fn alias_name(&self) -> Option<&str> {
        match self {
            Self::Table { alias, .. } | Self::View { alias, .. } | Self::Cte { alias, .. } => {
                alias.as_deref()
            }
            Self::Raw { alias, .. } | Self::Subquery { alias, .. } => Some(alias),
        }
    }

    pub fn base_name(&self) -> &str {
        match self {
            Self::Table { name, .. } | Self::View { name, .. } | Self::Cte { name, .. } => name,
            Self::Raw { alias, .. } | Self::Subquery { alias, .. } => alias,
        }
    }

    pub fn sql_qualifier(&self) -> &str {
        self.alias_name().unwrap_or_else(|| self.base_name())
    }
}

/// Queryable source metadata used by builders and JSON search validation.
///
/// A dataset is intentionally broader than a table: it can point at a table,
/// view, CTE, or raw source and carries the field metadata, limits, and
/// capabilities needed to validate dynamic requests before rendering SQL.
#[derive(Clone, Debug, PartialEq)]
pub struct Dataset {
    pub api_name: Cow<'static, str>,
    pub source: Source,
    pub fields: Cow<'static, [Field]>,
    pub default_limit: u32,
    pub max_limit: u32,
}

impl Dataset {
    pub fn table(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(name.clone(), Source::table(name))
    }

    pub fn static_table(name: &'static str) -> Self {
        Self::new_static(name, Source::static_table(name))
    }

    pub fn view(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(name.clone(), Source::view(name))
    }

    pub fn static_view(name: &'static str) -> Self {
        Self::new_static(name, Source::static_view(name))
    }

    pub fn raw(sql: impl Into<String>, alias: impl Into<String>) -> Self {
        let alias = alias.into();
        Self::new(alias.clone(), Source::raw(sql, alias))
    }

    pub fn subquery(query: impl Into<QueryExpr>, alias: impl Into<String>) -> Self {
        let alias = alias.into();
        Self::new(alias.clone(), Source::subquery(query, alias))
    }

    pub fn cte(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(name.clone(), Source::cte(name))
    }

    pub fn new(api_name: impl Into<String>, source: Source) -> Self {
        Self {
            api_name: Cow::Owned(api_name.into()),
            source,
            fields: Cow::Borrowed(&[]),
            default_limit: 100,
            max_limit: 1000,
        }
    }

    pub fn new_static(api_name: &'static str, source: Source) -> Self {
        Self {
            api_name: Cow::Borrowed(api_name),
            source,
            fields: Cow::Borrowed(&[]),
            default_limit: 100,
            max_limit: 1000,
        }
    }

    pub fn field(mut self, field: Field) -> Self {
        self.fields.to_mut().push(field);
        self
    }

    pub fn fields<I>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = Field>,
    {
        self.fields.to_mut().extend(fields);
        self
    }

    pub fn static_fields(mut self, fields: &'static [Field]) -> Self {
        self.fields = Cow::Borrowed(fields);
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

/// Alias-aware field accessor used by generated schema modules.
///
/// `Relation` is not an ORM relationship. It wraps a dataset alias so generated
/// helpers can return qualified `FieldRef`s for joins, for example
/// `orders::table().alias("o").id()`.
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
    pub lateral: bool,
}

impl Join {
    pub fn new(kind: JoinKind, dataset: impl Into<Dataset>, on: impl Into<Expr>) -> Self {
        Self {
            kind,
            dataset: dataset.into(),
            on: Some(on.into()),
            lateral: false,
        }
    }

    pub fn lateral(kind: JoinKind, dataset: impl Into<Dataset>, on: impl Into<Expr>) -> Self {
        Self {
            kind,
            dataset: dataset.into(),
            on: Some(on.into()),
            lateral: true,
        }
    }

    pub fn cross(dataset: impl Into<Dataset>) -> Self {
        Self {
            kind: JoinKind::Cross,
            dataset: dataset.into(),
            on: None,
            lateral: false,
        }
    }

    pub fn cross_lateral(dataset: impl Into<Dataset>) -> Self {
        Self {
            kind: JoinKind::Cross,
            dataset: dataset.into(),
            on: None,
            lateral: true,
        }
    }
}

impl From<Source> for Dataset {
    fn from(source: Source) -> Self {
        let api_name: Cow<'static, str> = match &source {
            Source::Table { name, .. } | Source::View { name, .. } | Source::Cte { name, .. } => {
                name.clone()
            }
            Source::Raw { alias, .. } | Source::Subquery { alias, .. } => alias.clone(),
        };
        Self {
            api_name,
            source,
            fields: Cow::Borrowed(&[]),
            default_limit: 100,
            max_limit: 1000,
        }
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
    Query(Box<QueryExpr>),
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

impl From<QueryExpr> for CteBody {
    fn from(value: QueryExpr) -> Self {
        Self::Query(Box::new(value))
    }
}

impl From<SelectQuery> for CteBody {
    fn from(value: SelectQuery) -> Self {
        QueryExpr::from(value).into()
    }
}

impl From<SelectBuilder> for CteBody {
    fn from(value: SelectBuilder) -> Self {
        QueryExpr::from(value).into()
    }
}

impl From<SetQuery> for CteBody {
    fn from(value: SetQuery) -> Self {
        QueryExpr::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::types::FieldType;
    use pretty_assertions::assert_eq;

    use super::*;

    const STATIC_FIELDS: &[Field] = &[
        Field::new("id", FieldType::Uuid),
        Field::new("email", FieldType::Text),
    ];

    fn assert_static_fields(dataset: &Dataset) {
        match &dataset.fields {
            Cow::Borrowed(fields) => assert!(
                std::ptr::eq(*fields, STATIC_FIELDS),
                "dataset should keep the generated static field slice borrowed"
            ),
            Cow::Owned(_) => panic!("dataset should not allocate generated static fields"),
        }
    }

    #[test]
    fn static_dataset_fields_stay_borrowed_until_mutated() {
        let dataset = Dataset::static_table("users").static_fields(STATIC_FIELDS);
        assert_eq!(dataset.api_name, "users");
        assert_static_fields(&dataset);

        let cloned = dataset.clone();
        assert_static_fields(&cloned);

        let aliased = dataset.clone().alias("u");
        assert_eq!(aliased.source_alias(), Some("u"));
        assert_static_fields(&aliased);

        let extended = dataset.field(Field::new("name", FieldType::Text));
        assert!(matches!(extended.fields, Cow::Owned(_)));
        assert_eq!(extended.fields.len(), 3);
        assert_eq!(STATIC_FIELDS.len(), 2);
    }

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
