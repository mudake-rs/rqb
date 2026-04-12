use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::expr::{ColumnOperator, Expr, Operator, Sort, SortDir, SubqueryOperator};
use crate::request::SelectQuery;
use crate::value::Value;

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

macro_rules! delegate_value_ops {
    ($($method:ident),* $(,)?) => {
        $(
            pub fn $method(self, value: impl Into<Value>) -> Expr {
                FieldRef::from(self).$method(value)
            }
        )*
    };
}

macro_rules! delegate_col_ops {
    ($($method:ident),* $(,)?) => {
        $(
            pub fn $method(self, right: impl Into<FieldRef>) -> Expr {
                FieldRef::from(self).$method(right)
            }
        )*
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElemType {
    Text,
    Int,
    BigInt,
    Float,
    Numeric,
    Bool,
    Uuid,
    Timestamp,
    Date,
    Enum(EnumType),
}

impl ElemType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Int => "int",
            Self::BigInt => "bigint",
            Self::Float => "float",
            Self::Numeric => "numeric",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Timestamp => "timestamp",
            Self::Date => "date",
            Self::Enum(enum_type) => enum_type.name,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumType {
    pub schema: Option<&'static str>,
    pub name: &'static str,
    pub variants: &'static [&'static str],
}

impl EnumType {
    pub const fn new(
        schema: Option<&'static str>,
        name: &'static str,
        variants: &'static [&'static str],
    ) -> Self {
        Self {
            schema,
            name,
            variants,
        }
    }

    pub fn contains(self, value: &str) -> bool {
        self.variants.contains(&value)
    }

    pub fn display_name(self) -> String {
        match self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.to_owned(),
        }
    }

    pub fn allowed_values(self) -> String {
        self.variants.join(", ")
    }
}

pub trait DbEnum: Copy {
    const TYPE: EnumType;

    fn as_db_str(self) -> &'static str;
}

impl<T> From<T> for Value
where
    T: DbEnum,
{
    fn from(value: T) -> Self {
        Self::String(value.as_db_str().to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldType {
    Text,
    Integer,
    BigInt,
    Float,
    Numeric,
    Bool,
    Uuid,
    Timestamp,
    Date,
    Jsonb,
    Enum(EnumType),
    Array(ElemType),
}

impl FieldType {
    pub fn is_jsonb(self) -> bool {
        matches!(self, Self::Jsonb)
    }

    pub fn is_array(self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Integer | Self::BigInt | Self::Float | Self::Numeric
        )
    }

    pub fn is_temporal(self) -> bool {
        matches!(self, Self::Timestamp | Self::Date)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer => "integer",
            Self::BigInt => "bigint",
            Self::Float => "float",
            Self::Numeric => "numeric",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Timestamp => "timestamp",
            Self::Date => "date",
            Self::Jsonb => "jsonb",
            Self::Enum(enum_type) => enum_type.name,
            Self::Array(elem) => match elem {
                ElemType::Text => "text[]",
                ElemType::Int => "int[]",
                ElemType::BigInt => "bigint[]",
                ElemType::Float => "float[]",
                ElemType::Numeric => "numeric[]",
                ElemType::Bool => "bool[]",
                ElemType::Uuid => "uuid[]",
                ElemType::Timestamp => "timestamp[]",
                ElemType::Date => "date[]",
                ElemType::Enum(_) => "enum[]",
            },
        }
    }

    pub fn enum_type(self) -> Option<EnumType> {
        match self {
            Self::Enum(enum_type) => Some(enum_type),
            Self::Array(ElemType::Enum(enum_type)) => Some(enum_type),
            _ => None,
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonPathPolicy {
    Deny,
    Dynamic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub selectable: bool,
    pub sortable: bool,
    pub filterable: bool,
    pub json_path: JsonPathPolicy,
    pub text_search: TextSearchConfig,
}

impl Capabilities {
    pub const fn all() -> Self {
        Self {
            selectable: true,
            sortable: true,
            filterable: true,
            json_path: JsonPathPolicy::Deny,
            text_search: TextSearchConfig::None,
        }
    }

    pub const fn hidden() -> Self {
        Self {
            selectable: false,
            sortable: false,
            filterable: false,
            json_path: JsonPathPolicy::Deny,
            text_search: TextSearchConfig::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSearchConfig {
    None,
    Config(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub api_name: &'static str,
    pub db_name: &'static str,
    pub ty: FieldType,
    pub caps: Capabilities,
}

impl Field {
    pub const fn new(name: &'static str, ty: FieldType) -> Self {
        Self::mapped(name, name, ty)
    }

    pub const fn mapped(api_name: &'static str, db_name: &'static str, ty: FieldType) -> Self {
        Self {
            api_name,
            db_name,
            ty,
            caps: Capabilities::all(),
        }
    }

    pub const fn selectable(mut self, selectable: bool) -> Self {
        self.caps.selectable = selectable;
        self
    }

    pub const fn sortable(mut self, sortable: bool) -> Self {
        self.caps.sortable = sortable;
        self
    }

    pub const fn filterable(mut self, filterable: bool) -> Self {
        self.caps.filterable = filterable;
        self
    }

    pub const fn json_paths(mut self, policy: JsonPathPolicy) -> Self {
        self.caps.json_path = policy;
        self
    }

    pub const fn text_search(mut self, config: &'static str) -> Self {
        self.caps.text_search = TextSearchConfig::Config(config);
        self
    }

    pub fn path(self, path: impl Into<String>) -> FieldRef {
        FieldRef::Known {
            qualifier: None,
            field: self,
            path: vec![path.into()],
            alias: None,
        }
    }

    pub fn on(self, qualifier: impl Into<String>) -> FieldRef {
        FieldRef::Known {
            qualifier: Some(qualifier.into()),
            field: self,
            path: Vec::new(),
            alias: None,
        }
    }

    pub fn alias(self, alias: impl Into<String>) -> FieldRef {
        FieldRef::from(self).alias(alias)
    }

    delegate_value_ops!(
        eq,
        ne,
        gt,
        gte,
        lt,
        lte,
        contains,
        not_contains,
        not_in,
        not_starts_with,
        not_ends_with,
        is_distinct_from,
        is_not_distinct_from,
        starts_with,
        ends_with,
        is_in,
        contains_any,
        contains_all,
        elem_match,
        has,
        not_has,
        key_exists,
        keys_exist_any,
        keys_exist_all,
        regex,
        not_regex,
        search,
    );

    pub fn between(self, low: impl Into<Value>, high: impl Into<Value>) -> Expr {
        FieldRef::from(self).between(low, high)
    }

    pub fn not_between(self, low: impl Into<Value>, high: impl Into<Value>) -> Expr {
        FieldRef::from(self).not_between(low, high)
    }

    pub fn is_null(self) -> Expr {
        FieldRef::from(self).is_null()
    }

    pub fn is_not_null(self) -> Expr {
        FieldRef::from(self).is_not_null()
    }

    pub fn is_empty(self) -> Expr {
        FieldRef::from(self).is_empty()
    }

    pub fn is_not_empty(self) -> Expr {
        FieldRef::from(self).is_not_empty()
    }

    delegate_col_ops!(eq_col, ne_col, gt_col, gte_col, lt_col, lte_col);

    pub fn in_subquery(self, query: impl Into<SelectQuery>) -> Expr {
        FieldRef::from(self).in_subquery(query)
    }

    pub fn not_in_subquery(self, query: impl Into<SelectQuery>) -> Expr {
        FieldRef::from(self).not_in_subquery(query)
    }

    pub fn asc(self) -> Sort {
        Sort::new(self, SortDir::Asc)
    }

    pub fn desc(self) -> Sort {
        Sort::new(self, SortDir::Desc)
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedField {
    pub api_name: String,
    pub db_name: String,
    pub ty: FieldType,
    pub caps: Capabilities,
    pub json_path: Vec<String>,
    pub qualifier: Option<String>,
    pub explicit_qualifier: Option<String>,
    pub alias: Option<String>,
}

impl ResolvedField {
    pub fn display_name(&self) -> String {
        let name = if self.json_path.is_empty() {
            self.api_name.clone()
        } else {
            format!("{}.{}", self.api_name, self.json_path.join("."))
        };
        match &self.explicit_qualifier {
            Some(qualifier) => format!("{qualifier}.{name}"),
            None => name,
        }
    }

    pub fn is_json_path(&self) -> bool {
        !self.json_path.is_empty()
    }

    pub fn output_alias(&self) -> String {
        if let Some(alias) = &self.alias {
            return alias.clone();
        }
        match &self.explicit_qualifier {
            Some(qualifier) => format!("{qualifier}_{}", self.api_name),
            None => self.api_name.clone(),
        }
    }

    pub fn object_key(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.api_name)
    }
}
