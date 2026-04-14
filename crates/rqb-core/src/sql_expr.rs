use crate::expr::Expr;
use crate::field::{Field, FieldRef};
use crate::raw::RawSql;
use crate::types::FieldType;
use crate::value::Value;

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum SqlExpr {
    Field(FieldRef),
    Excluded(FieldRef),
    Value(Value),
    Raw {
        raw: RawSql,
        ty: FieldType,
    },
    Function {
        name: String,
        args: Vec<SqlExpr>,
        ty: FieldType,
    },
    BuiltinFunction {
        function: BuiltinFunction,
        args: Vec<SqlExpr>,
    },
    JsonAccess {
        expr: Box<SqlExpr>,
        path: JsonAccessPath,
        text: bool,
    },
    Coalesce(Vec<SqlExpr>),
    Case {
        branches: Vec<CaseBranch>,
        otherwise: Box<SqlExpr>,
    },
    Cast {
        expr: Box<SqlExpr>,
        ty: FieldType,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFunction {
    Lower,
    Upper,
    Trim,
    Length,
    Now,
    DateTrunc,
    GenRandomUuid,
    Nullif,
    Greatest,
    Least,
}

impl BuiltinFunction {
    pub fn sql_name(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Upper => "upper",
            Self::Trim => "trim",
            Self::Length => "length",
            Self::Now => "now",
            Self::DateTrunc => "date_trunc",
            Self::GenRandomUuid => "gen_random_uuid",
            Self::Nullif => "NULLIF",
            Self::Greatest => "GREATEST",
            Self::Least => "LEAST",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionNameStyle {
    Quoted,
    Raw,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonAccessPath {
    Key(String),
    Index(i32),
    Path(Vec<String>),
}

impl SqlExpr {
    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        SelectItem {
            expr: self,
            alias: alias.into(),
        }
    }

    pub fn cast(self, ty: FieldType) -> Self {
        Self::Cast {
            expr: Box::new(self),
            ty,
        }
    }

    pub fn json(self, key: impl Into<String>) -> Self {
        self.json_access(JsonAccessPath::Key(key.into()), false)
    }

    pub fn json_text(self, key: impl Into<String>) -> Self {
        self.json_access(JsonAccessPath::Key(key.into()), true)
    }

    pub fn json_index(self, index: i32) -> Self {
        self.json_access(JsonAccessPath::Index(index), false)
    }

    pub fn json_index_text(self, index: i32) -> Self {
        self.json_access(JsonAccessPath::Index(index), true)
    }

    pub fn json_path<I, S>(self, path: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.json_access(
            JsonAccessPath::Path(path.into_iter().map(Into::into).collect()),
            false,
        )
    }

    pub fn json_path_text<I, S>(self, path: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.json_access(
            JsonAccessPath::Path(path.into_iter().map(Into::into).collect()),
            true,
        )
    }

    fn json_access(self, path: JsonAccessPath, text: bool) -> Self {
        Self::JsonAccess {
            expr: Box::new(self),
            path,
            text,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct SelectItem {
    pub expr: SqlExpr,
    pub alias: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseBranch {
    pub condition: Expr,
    pub value: SqlExpr,
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct FunctionBuilder {
    name: String,
    args: Vec<SqlExpr>,
}

impl FunctionBuilder {
    pub fn returns(self, ty: FieldType) -> SqlExpr {
        SqlExpr::Function {
            name: self.name,
            args: self.args,
            ty,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct CaseThenBuilder {
    branches: Vec<CaseBranch>,
    condition: Expr,
}

impl CaseThenBuilder {
    pub fn then(self, value: impl IntoSqlExpr) -> CaseBuilder {
        let mut branches = self.branches;
        branches.push(CaseBranch {
            condition: self.condition,
            value: value.into_sql_expr(),
        });
        CaseBuilder { branches }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct CaseBuilder {
    branches: Vec<CaseBranch>,
}

impl CaseBuilder {
    pub fn when(self, condition: impl Into<Expr>) -> CaseThenBuilder {
        CaseThenBuilder {
            branches: self.branches,
            condition: condition.into(),
        }
    }

    pub fn otherwise(self, value: impl IntoSqlExpr) -> SqlExpr {
        SqlExpr::Case {
            branches: self.branches,
            otherwise: Box::new(value.into_sql_expr()),
        }
    }
}

pub trait IntoSqlExpr {
    fn into_sql_expr(self) -> SqlExpr;

    fn json(self, key: impl Into<String>) -> SqlExpr
    where
        Self: Sized,
    {
        self.into_sql_expr().json(key)
    }

    fn json_text(self, key: impl Into<String>) -> SqlExpr
    where
        Self: Sized,
    {
        self.into_sql_expr().json_text(key)
    }

    fn json_index(self, index: i32) -> SqlExpr
    where
        Self: Sized,
    {
        self.into_sql_expr().json_index(index)
    }

    fn json_index_text(self, index: i32) -> SqlExpr
    where
        Self: Sized,
    {
        self.into_sql_expr().json_index_text(index)
    }

    fn json_path<I, S>(self, path: I) -> SqlExpr
    where
        Self: Sized,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.into_sql_expr().json_path(path)
    }

    fn json_path_text<I, S>(self, path: I) -> SqlExpr
    where
        Self: Sized,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.into_sql_expr().json_path_text(path)
    }
}

impl IntoSqlExpr for SqlExpr {
    fn into_sql_expr(self) -> SqlExpr {
        self
    }
}

impl IntoSqlExpr for FieldRef {
    fn into_sql_expr(self) -> SqlExpr {
        SqlExpr::Field(self)
    }
}

impl IntoSqlExpr for Field {
    fn into_sql_expr(self) -> SqlExpr {
        SqlExpr::Field(self.into())
    }
}

impl IntoSqlExpr for Value {
    fn into_sql_expr(self) -> SqlExpr {
        SqlExpr::Value(self)
    }
}

macro_rules! impl_sql_expr_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoSqlExpr for $ty {
                fn into_sql_expr(self) -> SqlExpr {
                    SqlExpr::Value(self.into())
                }
            }
        )*
    };
}

impl_sql_expr_value!(
    (),
    bool,
    i8,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    f32,
    f64,
    &str,
    String,
    serde_json::Value,
);

pub fn coalesce<I, E>(exprs: I) -> SqlExpr
where
    I: IntoIterator<Item = E>,
    E: IntoSqlExpr,
{
    SqlExpr::Coalesce(exprs.into_iter().map(IntoSqlExpr::into_sql_expr).collect())
}

pub fn cast(expr: impl IntoSqlExpr, ty: FieldType) -> SqlExpr {
    expr.into_sql_expr().cast(ty)
}

pub fn func<I, E>(name: impl Into<String>, args: I) -> FunctionBuilder
where
    I: IntoIterator<Item = E>,
    E: IntoSqlExpr,
{
    FunctionBuilder {
        name: name.into(),
        args: args.into_iter().map(IntoSqlExpr::into_sql_expr).collect(),
    }
}

fn builtin_function<I, E>(function: BuiltinFunction, args: I) -> SqlExpr
where
    I: IntoIterator<Item = E>,
    E: IntoSqlExpr,
{
    SqlExpr::BuiltinFunction {
        function,
        args: args.into_iter().map(IntoSqlExpr::into_sql_expr).collect(),
    }
}

pub fn lower(expr: impl IntoSqlExpr) -> SqlExpr {
    builtin_function(BuiltinFunction::Lower, [expr.into_sql_expr()])
}

pub fn upper(expr: impl IntoSqlExpr) -> SqlExpr {
    builtin_function(BuiltinFunction::Upper, [expr.into_sql_expr()])
}

pub fn trim(expr: impl IntoSqlExpr) -> SqlExpr {
    builtin_function(BuiltinFunction::Trim, [expr.into_sql_expr()])
}

pub fn length(expr: impl IntoSqlExpr) -> SqlExpr {
    builtin_function(BuiltinFunction::Length, [expr.into_sql_expr()])
}

pub fn now() -> SqlExpr {
    builtin_function(BuiltinFunction::Now, std::iter::empty::<SqlExpr>())
}

pub fn date_trunc(part: impl IntoSqlExpr, source: impl IntoSqlExpr) -> SqlExpr {
    builtin_function(
        BuiltinFunction::DateTrunc,
        [part.into_sql_expr(), source.into_sql_expr()],
    )
}

pub fn gen_random_uuid() -> SqlExpr {
    builtin_function(
        BuiltinFunction::GenRandomUuid,
        std::iter::empty::<SqlExpr>(),
    )
}

pub fn nullif(left: impl IntoSqlExpr, right: impl IntoSqlExpr) -> SqlExpr {
    builtin_function(
        BuiltinFunction::Nullif,
        [left.into_sql_expr(), right.into_sql_expr()],
    )
}

pub fn greatest<I, E>(exprs: I) -> SqlExpr
where
    I: IntoIterator<Item = E>,
    E: IntoSqlExpr,
{
    builtin_function(BuiltinFunction::Greatest, exprs)
}

pub fn least<I, E>(exprs: I) -> SqlExpr
where
    I: IntoIterator<Item = E>,
    E: IntoSqlExpr,
{
    builtin_function(BuiltinFunction::Least, exprs)
}

pub fn raw_expr(raw: RawSql, ty: FieldType) -> SqlExpr {
    SqlExpr::Raw { raw, ty }
}

pub fn excluded(field: impl Into<FieldRef>) -> SqlExpr {
    SqlExpr::Excluded(field.into())
}

pub fn case_when(condition: impl Into<Expr>) -> CaseThenBuilder {
    CaseThenBuilder {
        branches: Vec::new(),
        condition: condition.into(),
    }
}
