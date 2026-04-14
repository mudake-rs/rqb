use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map as JsonMap, Value as JsonValue};

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

#[derive(Clone, Debug, PartialEq)]
pub struct LogicalExpr {
    pub logical: LogicalOp,
    pub predicates: Vec<Expr>,
}

impl Serialize for LogicalExpr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_logical_expr(self, serializer)
    }
}

impl<'de> Deserialize<'de> for LogicalExpr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = JsonValue::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("logical expression must be a JSON object"))?;
        deserialize_logical_expr(object)
            .map_err(D::Error::custom)?
            .ok_or_else(|| {
                D::Error::custom("logical expression must contain `and`, `or`, or `not`")
            })
    }
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

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum Expr {
    Predicate(Predicate),
    ColumnPredicate(ColumnPredicate),
    Logical(LogicalExpr),
    Subquery(SubqueryPredicate),
    Exists(ExistsPredicate),
    Raw(RawSql),
}

impl Serialize for Expr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Predicate(predicate) => predicate.serialize(serializer),
            Self::ColumnPredicate(predicate) => predicate.serialize(serializer),
            Self::Logical(logical) => logical.serialize(serializer),
            Self::Subquery(_) | Self::Exists(_) | Self::Raw(_) => Err(serde::ser::Error::custom(
                "server-owned expressions cannot be serialized to JSON",
            )),
        }
    }
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
        return Err(
            "legacy logical expression uses `logical`/`predicates`; use `and`, `or`, or `not`"
                .to_owned(),
        );
    }

    if let Some(logical) = deserialize_logical_expr(object)? {
        return Ok(Expr::Logical(logical));
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

    Err("expression must contain `field`, `left`/`right`, or `and`/`or`/`not`".to_owned())
}

fn deserialize_logical_expr(
    object: &JsonMap<String, JsonValue>,
) -> Result<Option<LogicalExpr>, String> {
    let keys: Vec<&str> = ["and", "or", "not"]
        .into_iter()
        .filter(|key| object.contains_key(*key))
        .collect();

    let Some(key) = keys.first().copied() else {
        return Ok(None);
    };

    if keys.len() > 1 {
        return Err("logical expression must contain only one of `and`, `or`, or `not`".to_owned());
    }

    if object.len() != 1 {
        return Err(format!(
            "logical expression `{key}` cannot include extra fields"
        ));
    }

    let value = &object[key];
    let logical = match key {
        "and" => LogicalExpr {
            logical: LogicalOp::And,
            predicates: deserialize_logical_array("and", value)?,
        },
        "or" => LogicalExpr {
            logical: LogicalOp::Or,
            predicates: deserialize_logical_array("or", value)?,
        },
        "not" => {
            if value.is_array() {
                return Err("logical `not` expects a single expression object".to_owned());
            }
            LogicalExpr {
                logical: LogicalOp::Not,
                predicates: vec![deserialize_expr(value.clone())?],
            }
        }
        _ => unreachable!("logical keys are constrained above"),
    };

    Ok(Some(logical))
}

fn deserialize_logical_array(key: &str, value: &JsonValue) -> Result<Vec<Expr>, String> {
    if !value.is_array() {
        return Err(format!("logical `{key}` expects an array of expressions"));
    }
    serde_json::from_value::<Vec<Expr>>(value.clone())
        .map_err(|error| format!("invalid `{key}` logical expression: {error}"))
}

fn serialize_logical_expr<S>(logical: &LogicalExpr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(1))?;
    match logical.logical {
        LogicalOp::And => map.serialize_entry("and", &logical.predicates)?,
        LogicalOp::Or => map.serialize_entry("or", &logical.predicates)?,
        LogicalOp::Not => {
            let [predicate] = logical.predicates.as_slice() else {
                return Err(serde::ser::Error::custom(
                    "logical `not` must contain exactly one expression",
                ));
            };
            map.serialize_entry("not", predicate)?;
        }
    }
    map.end()
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
