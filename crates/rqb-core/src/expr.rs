use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::field::{Field, FieldRef};
use crate::query::QueryExpr;
use crate::raw::RawSql;
use crate::value::Value;

macro_rules! impl_as_str {
    ($ty:ident { $($variant:ident => $value:expr),* $(,)? }) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)*
                }
            }
        }

        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str((*self).as_str())
            }
        }
    };
}

mod operator;

pub(crate) use operator::OperatorCategory;
pub use operator::{ColumnOperator, Operator};

pub fn field(name: impl Into<String>) -> FieldRef {
    FieldRef::named(name)
}

pub fn all<I, E>(exprs: I) -> Expr
where
    I: IntoIterator<Item = E>,
    E: Into<Expr>,
{
    Expr::all(exprs)
}

pub fn any<I, E>(exprs: I) -> Expr
where
    I: IntoIterator<Item = E>,
    E: Into<Expr>,
{
    Expr::any(exprs)
}

pub fn not(expr: impl Into<Expr>) -> Expr {
    expr.into().not()
}

pub fn exists(query: impl Into<QueryExpr>) -> Expr {
    Expr::exists(query)
}

pub fn not_exists(query: impl Into<QueryExpr>) -> Expr {
    Expr::not_exists(query)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

impl_as_str!(LogicalOp {
    And => "and",
    Or => "or",
    Not => "not",
});

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Predicate {
    pub field: FieldRef,
    pub operator: Operator,
    #[serde(default = "default_predicate_value")]
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnPredicate {
    pub left: FieldRef,
    pub operator: ColumnOperator,
    pub right: FieldRef,
}

fn default_predicate_value() -> Value {
    Value::Null
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogicalExpr {
    pub logical: LogicalOp,
    pub predicates: Vec<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubqueryOperator {
    In,
    NotIn,
}

impl SubqueryOperator {
    pub fn as_sql(self) -> &'static str {
        match self {
            Self::In => "IN",
            Self::NotIn => "NOT IN",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubqueryPredicate {
    pub field: FieldRef,
    pub operator: SubqueryOperator,
    pub query: Box<QueryExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistsPredicate {
    pub query: Box<QueryExpr>,
    pub negated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
#[must_use]
pub enum Expr {
    Predicate(Predicate),
    ColumnPredicate(ColumnPredicate),
    Logical(LogicalExpr),
    #[serde(skip)]
    Subquery(SubqueryPredicate),
    #[serde(skip)]
    Exists(ExistsPredicate),
    #[serde(skip)]
    Raw(RawSql),
}

impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        deserialize_expr(value).map_err(D::Error::custom)
    }
}

fn deserialize_expr(value: serde_json::Value) -> Result<Expr, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "expression must be a JSON object".to_owned())?;

    if object.contains_key("logical") || object.contains_key("predicates") {
        if !object.contains_key("logical") {
            return Err("logical expression is missing `logical`".to_owned());
        }
        if !object.contains_key("predicates") {
            return Err("logical expression is missing `predicates`".to_owned());
        }
        return serde_json::from_value::<LogicalExpr>(value)
            .map(Expr::Logical)
            .map_err(|error| format!("invalid logical expression: {error}"));
    }

    if object.contains_key("field") {
        if !object.contains_key("operator") {
            return Err("predicate is missing `operator`".to_owned());
        }
        return serde_json::from_value::<Predicate>(value)
            .map(Expr::Predicate)
            .map_err(|error| format!("invalid predicate: {error}"));
    }

    if object.contains_key("left") || object.contains_key("right") {
        if !object.contains_key("left") {
            return Err("column predicate is missing `left`".to_owned());
        }
        if !object.contains_key("operator") {
            return Err("column predicate is missing `operator`".to_owned());
        }
        if !object.contains_key("right") {
            return Err("column predicate is missing `right`".to_owned());
        }
        return serde_json::from_value::<ColumnPredicate>(value)
            .map(Expr::ColumnPredicate)
            .map_err(|error| format!("invalid column predicate: {error}"));
    }

    Err("expression must contain `field`, `left`/`right`, or `logical`".to_owned())
}

impl Expr {
    pub fn predicate(
        field: impl Into<FieldRef>,
        operator: Operator,
        value: impl Into<Value>,
    ) -> Self {
        Self::Predicate(Predicate {
            field: field.into(),
            operator,
            value: value.into(),
        })
    }

    pub fn column_predicate(
        left: impl Into<FieldRef>,
        operator: ColumnOperator,
        right: impl Into<FieldRef>,
    ) -> Self {
        Self::ColumnPredicate(ColumnPredicate {
            left: left.into(),
            operator,
            right: right.into(),
        })
    }

    pub fn raw(raw: RawSql) -> Self {
        Self::Raw(raw)
    }

    pub fn subquery(
        field: impl Into<FieldRef>,
        operator: SubqueryOperator,
        query: impl Into<QueryExpr>,
    ) -> Self {
        Self::Subquery(SubqueryPredicate {
            field: field.into(),
            operator,
            query: Box::new(query.into()),
        })
    }

    pub fn exists(query: impl Into<QueryExpr>) -> Self {
        Self::Exists(ExistsPredicate {
            query: Box::new(query.into()),
            negated: false,
        })
    }

    pub fn not_exists(query: impl Into<QueryExpr>) -> Self {
        Self::Exists(ExistsPredicate {
            query: Box::new(query.into()),
            negated: true,
        })
    }

    pub fn all<I, E>(exprs: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Expr>,
    {
        Self::Logical(LogicalExpr {
            logical: LogicalOp::And,
            predicates: exprs.into_iter().map(Into::into).collect(),
        })
    }

    pub fn any<I, E>(exprs: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Expr>,
    {
        Self::Logical(LogicalExpr {
            logical: LogicalOp::Or,
            predicates: exprs.into_iter().map(Into::into).collect(),
        })
    }

    pub fn and(self, other: impl Into<Expr>) -> Self {
        Self::all([self, other.into()])
    }

    pub fn or(self, other: impl Into<Expr>) -> Self {
        Self::any([self, other.into()])
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Self {
        Self::Logical(LogicalExpr {
            logical: LogicalOp::Not,
            predicates: vec![self],
        })
    }
}

impl std::ops::Not for Expr {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Logical(LogicalExpr {
            logical: LogicalOp::Not,
            predicates: vec![self],
        })
    }
}

impl From<RawSql> for Expr {
    fn from(value: RawSql) -> Self {
        Self::Raw(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    Asc,
    Desc,
}

impl_as_str!(SortDir {
    Asc => "ASC",
    Desc => "DESC",
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NullsOrder {
    First,
    Last,
}

impl_as_str!(NullsOrder {
    First => "NULLS FIRST",
    Last => "NULLS LAST",
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[must_use]
pub struct Sort {
    pub field: FieldRef,
    #[serde(default = "default_sort_dir")]
    pub dir: SortDir,
    #[serde(default)]
    pub nulls: Option<NullsOrder>,
}

fn default_sort_dir() -> SortDir {
    SortDir::Asc
}

impl Sort {
    pub fn new(field: impl Into<FieldRef>, dir: SortDir) -> Self {
        Self {
            field: field.into(),
            dir,
            nulls: None,
        }
    }

    pub fn asc(field: impl Into<FieldRef>) -> Self {
        Self::new(field, SortDir::Asc)
    }

    pub fn desc(field: impl Into<FieldRef>) -> Self {
        Self::new(field, SortDir::Desc)
    }

    pub fn nulls_first(mut self) -> Self {
        self.nulls = Some(NullsOrder::First);
        self
    }

    pub fn nulls_last(mut self) -> Self {
        self.nulls = Some(NullsOrder::Last);
        self
    }
}

impl From<Field> for Sort {
    fn from(field: Field) -> Self {
        Self::asc(field)
    }
}
