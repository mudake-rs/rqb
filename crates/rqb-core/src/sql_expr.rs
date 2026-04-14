use crate::expr::{Expr, Sort};
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
    Window {
        function: WindowFunction,
        args: Vec<SqlExpr>,
        spec: WindowSpec,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFunction {
    RowNumber,
    Rank,
    DenseRank,
    Lag,
    Lead,
}

impl WindowFunction {
    pub fn sql_name(self) -> &'static str {
        match self {
            Self::RowNumber => "row_number",
            Self::Rank => "rank",
            Self::DenseRank => "dense_rank",
            Self::Lag => "lag",
            Self::Lead => "lead",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct WindowSpec {
    pub partition_by: Vec<FieldRef>,
    pub order_by: Vec<Sort>,
}

impl WindowSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn partition_by(mut self, field: impl Into<FieldRef>) -> Self {
        self.partition_by.push(field.into());
        self
    }

    pub fn partition_by_many<I, F>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FieldRef>,
    {
        self.partition_by.extend(fields.into_iter().map(Into::into));
        self
    }

    pub fn order_by(mut self, sort: impl Into<Sort>) -> Self {
        self.order_by.push(sort.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct WindowFunctionBuilder {
    function: WindowFunction,
    args: Vec<SqlExpr>,
}

impl WindowFunctionBuilder {
    pub fn over(self, spec: WindowSpec) -> SqlExpr {
        SqlExpr::Window {
            function: self.function,
            args: self.args,
            spec,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct OffsetWindowFunctionBuilder {
    function: WindowFunction,
    value: SqlExpr,
    offset: Option<SqlExpr>,
    default: Option<SqlExpr>,
}

impl OffsetWindowFunctionBuilder {
    pub fn offset(mut self, offset: impl IntoSqlExpr) -> Self {
        self.offset = Some(offset.into_sql_expr());
        self
    }

    pub fn default(mut self, value: impl IntoSqlExpr) -> Self {
        self.default = Some(value.into_sql_expr());
        self
    }

    pub fn over(self, spec: WindowSpec) -> SqlExpr {
        SqlExpr::Window {
            function: self.function,
            args: self.into_args(),
            spec,
        }
    }

    fn into_args(self) -> Vec<SqlExpr> {
        let mut args = vec![self.value];
        match (self.offset, self.default) {
            (Some(offset), Some(default)) => {
                args.push(offset);
                args.push(default);
            }
            (Some(offset), None) => args.push(offset),
            (None, Some(default)) => {
                args.push(1.into_sql_expr());
                args.push(default);
            }
            (None, None) => {}
        }
        args
    }
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
    u64,
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

fn window_function<I, E>(function: WindowFunction, args: I) -> WindowFunctionBuilder
where
    I: IntoIterator<Item = E>,
    E: IntoSqlExpr,
{
    WindowFunctionBuilder {
        function,
        args: args.into_iter().map(IntoSqlExpr::into_sql_expr).collect(),
    }
}

fn offset_window_function(
    expr: impl IntoSqlExpr,
    function: WindowFunction,
) -> OffsetWindowFunctionBuilder {
    OffsetWindowFunctionBuilder {
        function,
        value: expr.into_sql_expr(),
        offset: None,
        default: None,
    }
}

pub fn window() -> WindowSpec {
    WindowSpec::new()
}

pub fn partition_by(field: impl Into<FieldRef>) -> WindowSpec {
    WindowSpec::new().partition_by(field)
}

pub fn row_number() -> WindowFunctionBuilder {
    window_function(WindowFunction::RowNumber, std::iter::empty::<SqlExpr>())
}

pub fn rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::Rank, std::iter::empty::<SqlExpr>())
}

pub fn dense_rank() -> WindowFunctionBuilder {
    window_function(WindowFunction::DenseRank, std::iter::empty::<SqlExpr>())
}

pub fn lag(expr: impl IntoSqlExpr) -> OffsetWindowFunctionBuilder {
    offset_window_function(expr, WindowFunction::Lag)
}

pub fn lead(expr: impl IntoSqlExpr) -> OffsetWindowFunctionBuilder {
    offset_window_function(expr, WindowFunction::Lead)
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
