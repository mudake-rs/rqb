use crate::typed::{Meta, OrderItem, Param};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    IsDistinctFrom,
    IsNotDistinctFrom,
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
            Self::IsDistinctFrom => "IS DISTINCT FROM",
            Self::IsNotDistinctFrom => "IS NOT DISTINCT FROM",
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
            Self::IsDistinctFrom => "is_distinct_from",
            Self::IsNotDistinctFrom => "is_not_distinct_from",
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFunction {
    RowNumber,
    Rank,
    DenseRank,
    Lag,
    Lead,
}

#[derive(Clone, Debug, Default)]
pub struct WindowSpec {
    pub partition_by: Vec<ValueExpr>,
    pub order_by: Vec<OrderItem>,
}

#[derive(Clone, Debug)]
pub struct WindowFunctionBuilder {
    pub(super) function: WindowFunction,
    pub(super) args: Vec<ValueExpr>,
}

#[derive(Clone, Debug)]
pub struct OffsetWindowFunctionBuilder {
    pub(super) function: WindowFunction,
    pub(super) value: ValueExpr,
    pub(super) offset: Option<ValueExpr>,
    pub(super) default: Option<ValueExpr>,
}

#[derive(Clone, Debug)]
pub enum BoolExpr {
    Constant(bool),
    Compare {
        left: ValueExpr,
        op: BoolOp,
        right: ValueExpr,
    },
    IsNull {
        expr: ValueExpr,
        negated: bool,
    },
    InList {
        expr: ValueExpr,
        values: Vec<ValueExpr>,
        negated: bool,
    },
    InSubquery {
        expr: ValueExpr,
        query: Box<crate::typed::Stmt>,
        negated: bool,
    },
    Between {
        expr: ValueExpr,
        low: ValueExpr,
        high: ValueExpr,
        negated: bool,
    },
    Like {
        expr: ValueExpr,
        pattern: ValueExpr,
        case_insensitive: bool,
        negated: bool,
        escape: bool,
    },
    Regex {
        expr: ValueExpr,
        pattern: ValueExpr,
        case_insensitive: bool,
        negated: bool,
    },
    Infix {
        left: ValueExpr,
        op: &'static str,
        right: ValueExpr,
        negated: bool,
    },
    Any {
        value: ValueExpr,
        array: ValueExpr,
        negated: bool,
    },
    ArrayIsEmpty {
        expr: ValueExpr,
        negated: bool,
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
    Field {
        meta: Meta,
        qualifier: Option<String>,
    },
    Excluded(Meta),
    Param(Param),
    Function {
        name: &'static str,
        args: Vec<ValueExpr>,
    },
    Aggregate {
        name: &'static str,
        args: Vec<ValueExpr>,
        distinct: bool,
        order_by: Vec<OrderItem>,
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
    Window {
        function: WindowFunction,
        args: Vec<ValueExpr>,
        spec: WindowSpec,
    },
    Raw {
        sql: String,
        params: Vec<Param>,
    },
    Subquery(Box<crate::typed::Stmt>),
}
