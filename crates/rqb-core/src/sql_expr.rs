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
