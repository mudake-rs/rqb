use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::expr::{ColumnOperator, Expr, Operator, Sort, SortDir, SubqueryOperator};
use crate::request::SelectQuery;
use crate::value::Value;

use super::Field;

macro_rules! predicate_ops {
    ($($method:ident => $op:ident),* $(,)?) => {
        $(
            pub fn $method(self, value: impl Into<Value>) -> Expr {
                self.predicate(Operator::$op, value)
            }
        )*
    };
}

macro_rules! column_ops {
    ($($method:ident => $op:ident),* $(,)?) => {
        $(
            pub fn $method(self, right: impl Into<FieldRef>) -> Expr {
                self.column_predicate(ColumnOperator::$op, right)
            }
        )*
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldRef {
    Named {
        name: String,
        alias: Option<String>,
    },
    Known {
        qualifier: Option<String>,
        field: Field,
        path: Vec<String>,
        alias: Option<String>,
    },
}

impl FieldRef {
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named {
            name: name.into(),
            alias: None,
        }
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        let alias = Some(alias.into());
        match &mut self {
            Self::Named { alias: current, .. } | Self::Known { alias: current, .. } => {
                *current = alias;
            }
        }
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        match &mut self {
            Self::Named { name, .. } => {
                name.push('.');
                name.push_str(&path.into());
            }
            Self::Known { path: paths, .. } => {
                paths.push(path.into());
            }
        }
        self
    }

    pub fn on(mut self, qualifier: impl Into<String>) -> Self {
        let qualifier = qualifier.into();
        match &mut self {
            Self::Named { name, .. } => {
                *name = format!("{qualifier}.{name}");
            }
            Self::Known {
                qualifier: current, ..
            } => *current = Some(qualifier),
        }
        self
    }

    pub fn qualifier(&self) -> Option<&str> {
        match self {
            Self::Named { name, .. } => name.split_once('.').map(|(qualifier, _)| qualifier),
            Self::Known { qualifier, .. } => qualifier.as_deref(),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Named { name, .. } => name.clone(),
            Self::Known {
                qualifier,
                field,
                path,
                ..
            } if path.is_empty() => match qualifier {
                Some(qualifier) => format!("{qualifier}.{}", field.api_name),
                None => field.api_name.to_owned(),
            },
            Self::Known {
                qualifier,
                field,
                path,
                ..
            } => match qualifier {
                Some(qualifier) => {
                    format!("{qualifier}.{}.{}", field.api_name, path.join("."))
                }
                None => format!("{}.{}", field.api_name, path.join(".")),
            },
        }
    }

    pub fn predicate(self, operator: Operator, value: impl Into<Value>) -> Expr {
        Expr::predicate(self, operator, value)
    }

    predicate_ops!(
        eq => Equals,
        ne => NotEquals,
        gt => Gt,
        gte => Gte,
        lt => Lt,
        lte => Lte,
        contains => Contains,
        not_contains => NotContains,
        not_in => NotIn,
        not_starts_with => NotStartsWith,
        not_ends_with => NotEndsWith,
        is_distinct_from => IsDistinctFrom,
        is_not_distinct_from => IsNotDistinctFrom,
        starts_with => StartsWith,
        ends_with => EndsWith,
        is_in => In,
        contains_any => ArrayContainsAny,
        contains_all => ArrayContainsAll,
        elem_match => ArrayElemMatch,
        has => ArrayContains,
        not_has => ArrayNotContains,
        key_exists => JsonKeyExists,
        keys_exist_any => JsonKeysExistAny,
        keys_exist_all => JsonKeysExistAll,
        contained_by => ContainedBy,
        overlaps => Overlaps,
        regex => Regex,
        not_regex => NotRegex,
        search => TextSearch,
    );

    pub fn between(self, low: impl Into<Value>, high: impl Into<Value>) -> Expr {
        self.predicate(
            Operator::Between,
            Value::Array(vec![low.into(), high.into()]),
        )
    }

    pub fn not_between(self, low: impl Into<Value>, high: impl Into<Value>) -> Expr {
        self.predicate(
            Operator::NotBetween,
            Value::Array(vec![low.into(), high.into()]),
        )
    }

    pub fn is_null(self) -> Expr {
        self.predicate(Operator::IsNull, Value::Null)
    }

    pub fn is_not_null(self) -> Expr {
        self.predicate(Operator::IsNotNull, Value::Null)
    }

    pub fn is_empty(self) -> Expr {
        self.predicate(Operator::ArrayIsEmpty, Value::Null)
    }

    pub fn is_not_empty(self) -> Expr {
        self.predicate(Operator::ArrayIsNotEmpty, Value::Null)
    }

    pub fn column_predicate(self, operator: ColumnOperator, right: impl Into<FieldRef>) -> Expr {
        Expr::column_predicate(self, operator, right)
    }

    column_ops!(
        eq_col => Equals,
        ne_col => NotEquals,
        gt_col => Gt,
        gte_col => Gte,
        lt_col => Lt,
        lte_col => Lte,
    );

    pub fn in_subquery(self, query: impl Into<SelectQuery>) -> Expr {
        Expr::subquery(self, SubqueryOperator::In, query)
    }

    pub fn not_in_subquery(self, query: impl Into<SelectQuery>) -> Expr {
        Expr::subquery(self, SubqueryOperator::NotIn, query)
    }

    pub fn asc(self) -> Sort {
        Sort::new(self, SortDir::Asc)
    }

    pub fn desc(self) -> Sort {
        Sort::new(self, SortDir::Desc)
    }
}

impl From<&str> for FieldRef {
    fn from(value: &str) -> Self {
        Self::named(value)
    }
}

impl From<String> for FieldRef {
    fn from(value: String) -> Self {
        Self::named(value)
    }
}

impl From<Field> for FieldRef {
    fn from(field: Field) -> Self {
        Self::Known {
            qualifier: None,
            field,
            path: Vec::new(),
            alias: None,
        }
    }
}

impl From<(Field, &str)> for FieldRef {
    fn from((field, qualifier): (Field, &str)) -> Self {
        field.on(qualifier)
    }
}

impl Serialize for FieldRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.display_name().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FieldRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::named)
    }
}
