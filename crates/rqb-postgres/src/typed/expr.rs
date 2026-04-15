use std::marker::PhantomData;

use sqlx::{Encode, Postgres, Type};

use crate::typed::{Meta, Param, SelectItem, raw};
use crate::{Error, Result};

#[derive(Debug, PartialEq, Eq)]
pub struct Field<T> {
    pub meta: &'static Meta,
    _ty: PhantomData<T>,
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Field<T> {}

impl<T> Field<T> {
    pub const fn new(meta: &'static Meta) -> Self {
        Self {
            meta,
            _ty: PhantomData,
        }
    }

    pub fn expr(self) -> ValueExpr {
        ValueExpr::Field(*self.meta)
    }

    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        self.expr().alias(alias)
    }

    pub fn set<V>(self, value: V) -> crate::typed::Assignment
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        crate::typed::Assignment {
            field: *self.meta,
            value: ValueExpr::Param(Param::typed(value.into())),
        }
    }

    pub fn eq<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Eq, value)
    }

    pub fn ne<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Ne, value)
    }

    pub fn gt<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Gt, value)
    }

    pub fn gte<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Gte, value)
    }

    pub fn lt<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Lt, value)
    }

    pub fn lte<V>(self, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.compare(BoolOp::Lte, value)
    }

    fn compare<V>(self, op: BoolOp, value: V) -> BoolExpr
    where
        V: Into<T>,
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        BoolExpr::Compare {
            left: ValueExpr::Field(*self.meta),
            op,
            right: ValueExpr::Param(Param::typed(value.into())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl BoolOp {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }

    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
        }
    }

    pub const fn requires_ordering(self) -> bool {
        matches!(self, Self::Gt | Self::Gte | Self::Lt | Self::Lte)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOp {
    Add,
    Sub,
    Mul,
    Div,
    Custom(&'static str),
}

#[derive(Clone, Debug)]
pub enum BoolExpr {
    Compare {
        left: ValueExpr,
        op: BoolOp,
        right: ValueExpr,
    },
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Not(Box<BoolExpr>),
    Exists(Box<crate::typed::Stmt>),
    Raw {
        sql: String,
        params: Vec<Param>,
    },
}

#[derive(Clone, Debug)]
pub enum ValueExpr {
    Field(Meta),
    Param(Param),
    Function {
        name: &'static str,
        args: Vec<ValueExpr>,
    },
    Aggregate {
        name: &'static str,
        args: Vec<ValueExpr>,
        filter: Option<Box<BoolExpr>>,
    },
    Case {
        branches: Vec<(BoolExpr, ValueExpr)>,
        else_: Option<Box<ValueExpr>>,
    },
    Cast {
        expr: Box<ValueExpr>,
        pg: &'static str,
    },
    Binary {
        left: Box<ValueExpr>,
        op: ValueOp,
        right: Box<ValueExpr>,
    },
    Raw {
        sql: String,
        params: Vec<Param>,
    },
    Subquery(Box<crate::typed::Stmt>),
}

impl BoolExpr {
    pub fn and(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::And(exprs.into_iter().collect())
    }

    pub fn or(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::Or(exprs.into_iter().collect())
    }

    pub fn negate(expr: BoolExpr) -> Self {
        Self::Not(Box::new(expr))
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Compare { left, op, right } => {
                validate_compare(left, *op)?;
                left.validate()?;
                right.validate()
            }
            Self::And(exprs) | Self::Or(exprs) => {
                if exprs.is_empty() {
                    return Err(Error::EmptyTypedLogical {
                        logical: match self {
                            Self::And(_) => "and",
                            Self::Or(_) => "or",
                            _ => unreachable!(),
                        }
                        .to_owned(),
                    });
                }
                for expr in exprs {
                    expr.validate()?;
                }
                Ok(())
            }
            Self::Not(expr) => expr.validate(),
            Self::Exists(stmt) => stmt.validate(),
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
        }
    }

    pub fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Compare { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::And(exprs) | Self::Or(exprs) => {
                for expr in exprs {
                    expr.collect_params(params);
                }
            }
            Self::Not(expr) => expr.collect_params(params),
            Self::Exists(stmt) => stmt.collect_params(params),
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
        }
    }
}

impl ValueExpr {
    pub fn param<T>(value: T) -> Self
    where
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        Self::Param(Param::typed(value))
    }

    pub fn alias(self, alias: impl Into<String>) -> SelectItem {
        SelectItem {
            expr: self,
            alias: Some(alias.into()),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Aggregate { filter, args, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                if let Some(filter) = filter {
                    filter.validate()?;
                }
                Ok(())
            }
            Self::Function { args, .. } => {
                for arg in args {
                    arg.validate()?;
                }
                Ok(())
            }
            Self::Case { branches, else_ } => {
                for (when, then) in branches {
                    when.validate()?;
                    then.validate()?;
                }
                if let Some(else_) = else_ {
                    else_.validate()?;
                }
                Ok(())
            }
            Self::Cast { expr, .. } => expr.validate(),
            Self::Binary { left, right, .. } => {
                left.validate()?;
                right.validate()
            }
            Self::Raw { sql, params } => raw::validate_bind_count(sql, params.len()),
            Self::Subquery(stmt) => stmt.validate(),
            Self::Field(_) | Self::Param(_) => Ok(()),
        }
    }

    pub(crate) fn field_meta(&self) -> Option<&Meta> {
        match self {
            Self::Field(meta) => Some(meta),
            _ => None,
        }
    }

    pub fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Param(param) => params.push(param.clone()),
            Self::Function { args, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
            }
            Self::Aggregate { args, filter, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
                if let Some(filter) = filter {
                    filter.collect_params(params);
                }
            }
            Self::Case { branches, else_ } => {
                for (when, then) in branches {
                    when.collect_params(params);
                    then.collect_params(params);
                }
                if let Some(else_) = else_ {
                    else_.collect_params(params);
                }
            }
            Self::Cast { expr, .. } => expr.collect_params(params),
            Self::Binary { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
            Self::Subquery(stmt) => stmt.collect_params(params),
            Self::Field(_) => {}
        }
    }
}

impl<T> From<Field<T>> for ValueExpr {
    fn from(field: Field<T>) -> Self {
        field.expr()
    }
}

impl From<Param> for ValueExpr {
    fn from(param: Param) -> Self {
        Self::Param(param)
    }
}

fn validate_compare(left: &ValueExpr, op: BoolOp) -> Result<()> {
    let Some(meta) = left.field_meta() else {
        return Ok(());
    };
    let supported = if op.requires_ordering() {
        meta.ops.ordering
    } else {
        meta.ops.equality
    };
    if supported {
        return Ok(());
    }
    Err(Error::InvalidTypedOperator {
        field: meta.api.to_owned(),
        operator: op.as_name().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::typed::{BoolOp, Field, JsonKind, Meta, OpSet, Param, Params, ValueExpr};

    #[test]
    fn field_t_erases_to_bool_expr_with_sqlx_param() {
        static ID_META: Meta = Meta::new("id", "id", "uuid")
            .ops(OpSet::equality())
            .json(JsonKind::Uuid);
        const ID: Field<Uuid> = Field::new(&ID_META);

        let expr = ID.eq(Uuid::nil());
        expr.validate().unwrap();

        let mut raw_params = Vec::new();
        expr.collect_params(&mut raw_params);
        let params = Params::from_vec(raw_params);

        assert_eq!(params.len(), 1);
        assert!(params.debug_names()[0].ends_with("uuid::Uuid"));
    }

    #[test]
    fn field_is_copy_without_requiring_t_to_be_copy() {
        static EMAIL_META: Meta = Meta::new("email", "email", "text").ops(OpSet::ordered());
        const EMAIL: Field<String> = Field::new(&EMAIL_META);

        let field = EMAIL;
        let _first = field.expr();
        let _second = field.expr();
    }

    #[test]
    fn operator_validation_uses_meta_not_rust_type_traits() {
        static PAYLOAD_META: Meta = Meta::new("payload", "payload", "jsonb")
            .json(JsonKind::Jsonb)
            .ops(OpSet::equality());
        const PAYLOAD: Field<serde_json::Value> = Field::new(&PAYLOAD_META);

        let err = PAYLOAD
            .gt(serde_json::json!({ "n": 1 }))
            .validate()
            .unwrap_err();

        assert!(matches!(
            err,
            crate::Error::InvalidTypedOperator { field, operator }
                if field == "payload" && operator == "gt"
        ));
    }

    #[test]
    fn value_expr_is_separate_from_bool_expr() {
        static EMAIL_META: Meta = Meta::new("email", "email", "text")
            .ops(OpSet::ordered())
            .json(JsonKind::Text);
        const EMAIL: Field<String> = Field::new(&EMAIL_META);

        let lower = ValueExpr::Function {
            name: "lower",
            args: vec![EMAIL.expr()],
        };
        let filter = crate::typed::BoolExpr::Compare {
            left: lower,
            op: BoolOp::Eq,
            right: ValueExpr::Param(Param::typed("egor@example.com".to_owned())),
        };

        filter.validate().unwrap();
    }

    #[test]
    fn meta_defaults_to_no_typed_operators() {
        static SCORE_META: Meta = Meta::new("score", "score", "int4");
        const SCORE: Field<i32> = Field::new(&SCORE_META);

        let err = SCORE.eq(10).validate().unwrap_err();

        assert!(matches!(
            err,
            crate::Error::InvalidTypedOperator { field, operator }
                if field == "score" && operator == "eq"
        ));
    }

    #[test]
    fn raw_predicate_validates_bind_count() {
        let err = crate::typed::BoolExpr::Raw {
            sql: "score > ? and active = ?".to_owned(),
            params: vec![Param::typed(10_i32)],
        }
        .validate()
        .unwrap_err();

        assert!(matches!(
            err,
            crate::Error::RawBindMismatch {
                placeholders: 2,
                binds: 1
            }
        ));
    }
}
