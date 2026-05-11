use crate::{Meta, OrderItem, Param};

/// Binary comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    /// Equality (`=`).
    Eq,
    /// Inequality (`<>`).
    Ne,
    /// Greater-than (`>`).
    Gt,
    /// Greater-than-or-equal (`>=`).
    Gte,
    /// Less-than (`<`).
    Lt,
    /// Less-than-or-equal (`<=`).
    Lte,
    /// Null-safe inequality (`IS DISTINCT FROM`).
    IsDistinctFrom,
    /// Null-safe equality (`IS NOT DISTINCT FROM`).
    IsNotDistinctFrom,
}

/// Boolean truth test target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanTest {
    /// `TRUE`.
    True,
    /// `FALSE`.
    False,
    /// `UNKNOWN`.
    Unknown,
}

impl BooleanTest {
    /// Returns the SQL keyword for this test.
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::True => "TRUE",
            Self::False => "FALSE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl BoolOp {
    /// Returns the SQL operator token.
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

    /// Returns a stable operator name for diagnostics.
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

    /// Returns true when this operator requires ordered field capability.
    pub const fn requires_ordering(self) -> bool {
        matches!(self, Self::Gt | Self::Gte | Self::Lt | Self::Lte)
    }
}

/// Arithmetic or custom value operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOp {
    /// Addition (`+`).
    Add,
    /// Subtraction (`-`).
    Sub,
    /// Multiplication (`*`).
    Mul,
    /// Division (`/`).
    Div,
    /// Custom infix operator.
    Custom(&'static str),
}

impl ValueOp {
    /// Returns the SQL operator token.
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Custom(op) => op,
        }
    }
}

/// Supported SQL window function names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFunction {
    /// `row_number`.
    RowNumber,
    /// `rank`.
    Rank,
    /// `dense_rank`.
    DenseRank,
    /// `lag`.
    Lag,
    /// `lead`.
    Lead,
    /// `first_value`.
    FirstValue,
    /// `last_value`.
    LastValue,
    /// `nth_value`.
    NthValue,
    /// `ntile`.
    Ntile,
    /// `percent_rank`.
    PercentRank,
    /// `cume_dist`.
    CumeDist,
}

/// Window frame unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFrameKind {
    /// `ROWS`.
    Rows,
    /// `RANGE`.
    Range,
    /// `GROUPS`.
    Groups,
}

/// Window frame boundary.
#[derive(Clone, Debug)]
#[must_use]
pub enum FrameBound {
    /// `UNBOUNDED PRECEDING`.
    UnboundedPreceding,
    /// `n PRECEDING`.
    Preceding(Box<ValueExpr>),
    /// `CURRENT ROW`.
    CurrentRow,
    /// `n FOLLOWING`.
    Following(Box<ValueExpr>),
    /// `UNBOUNDED FOLLOWING`.
    UnboundedFollowing,
}

/// Window frame exclusion clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameExclude {
    /// `EXCLUDE CURRENT ROW`.
    CurrentRow,
    /// `EXCLUDE GROUP`.
    Group,
    /// `EXCLUDE TIES`.
    Ties,
    /// `EXCLUDE NO OTHERS`.
    NoOthers,
}

/// Window frame specification.
#[derive(Clone, Debug)]
#[must_use]
pub struct WindowFrame {
    /// Frame unit.
    pub kind: WindowFrameKind,
    /// Start boundary.
    pub start: FrameBound,
    /// Optional end boundary for `BETWEEN`.
    pub end: Option<FrameBound>,
    /// Optional exclusion clause.
    pub exclude: Option<FrameExclude>,
}

/// Window specification used by `OVER (...)`.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct WindowSpec {
    /// Partition expressions.
    pub partition_by: Vec<ValueExpr>,
    /// Ordering expressions.
    pub order_by: Vec<OrderItem>,
    /// Optional frame.
    pub frame: Option<Box<WindowFrame>>,
}

/// Builder for window functions without offset/default arguments.
#[derive(Clone, Debug)]
#[must_use]
pub struct WindowFunctionBuilder {
    pub(super) function: WindowFunction,
    pub(super) args: Vec<ValueExpr>,
}

/// Builder for `lag` / `lead` window functions.
#[derive(Clone, Debug)]
#[must_use]
pub struct OffsetWindowFunctionBuilder {
    pub(super) function: WindowFunction,
    pub(super) value: ValueExpr,
    pub(super) offset: Option<ValueExpr>,
    pub(super) default: Option<ValueExpr>,
}

/// Builder for SQL `CASE WHEN ... THEN ... ELSE ... END` expressions.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct CaseBuilder {
    pub(super) branches: Vec<(BoolExpr, ValueExpr)>,
}

/// Boolean expression AST.
#[derive(Clone, Debug)]
#[must_use]
pub enum BoolExpr {
    /// Boolean constant.
    Constant(bool),
    /// Binary comparison.
    Compare {
        /// Left side.
        left: ValueExpr,
        /// Comparison operator.
        op: BoolOp,
        /// Right side.
        right: ValueExpr,
    },
    /// `IS NULL` / `IS NOT NULL`.
    IsNull {
        /// Tested expression.
        expr: ValueExpr,
        /// Whether to render `IS NOT NULL`.
        negated: bool,
    },
    /// `IS TRUE` / `IS NOT TRUE` and related boolean tests.
    IsBoolean {
        /// Tested expression.
        expr: ValueExpr,
        /// Boolean test kind.
        test: BooleanTest,
        /// Whether to negate the test.
        negated: bool,
    },
    /// `IN (...)` / `NOT IN (...)`.
    InList {
        /// Tested expression.
        expr: ValueExpr,
        /// List values.
        values: Vec<ValueExpr>,
        /// Whether to render `NOT IN`.
        negated: bool,
    },
    /// `IN (subquery)` / `NOT IN (subquery)`.
    InSubquery {
        /// Tested expression.
        expr: ValueExpr,
        /// Subquery statement.
        query: Box<crate::Stmt>,
        /// Whether to render `NOT IN`.
        negated: bool,
    },
    /// `BETWEEN` / `NOT BETWEEN`.
    Between {
        /// Tested expression.
        expr: ValueExpr,
        /// Lower bound.
        low: ValueExpr,
        /// Upper bound.
        high: ValueExpr,
        /// Whether to render `NOT BETWEEN`.
        negated: bool,
    },
    /// `LIKE` / `ILIKE` predicate.
    Like {
        /// Tested expression.
        expr: ValueExpr,
        /// Pattern expression.
        pattern: ValueExpr,
        /// Whether to render `ILIKE`.
        case_insensitive: bool,
        /// Whether to negate the predicate.
        negated: bool,
        /// Whether to add `ESCAPE '\'`.
        escape: bool,
    },
    /// `SIMILAR TO` predicate.
    SimilarTo {
        /// Tested expression.
        expr: ValueExpr,
        /// Pattern expression.
        pattern: ValueExpr,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// PostgreSQL regex predicate.
    Regex {
        /// Tested expression.
        expr: ValueExpr,
        /// Pattern expression.
        pattern: ValueExpr,
        /// Whether to use case-insensitive regex matching.
        case_insensitive: bool,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// Custom typed infix predicate.
    Infix {
        /// Left expression.
        left: ValueExpr,
        /// SQL operator token.
        op: &'static str,
        /// Right expression.
        right: ValueExpr,
        /// Whether to wrap with `NOT`.
        negated: bool,
    },
    /// `value = ANY(array)` / negated form.
    Any {
        /// Value expression.
        value: ValueExpr,
        /// Array expression.
        array: ValueExpr,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// Array-empty predicate.
    ArrayIsEmpty {
        /// Array expression.
        expr: ValueExpr,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// Logical conjunction.
    And(Vec<BoolExpr>),
    /// Logical disjunction.
    Or(Vec<BoolExpr>),
    /// Logical negation.
    Not(Box<BoolExpr>),
    /// `EXISTS (subquery)`.
    Exists(Box<crate::Stmt>),
    /// Server-owned raw predicate.
    Raw {
        /// Raw SQL using rqb `?` placeholders.
        sql: String,
        /// Bind parameters for the raw SQL.
        params: Vec<Param>,
    },
}

/// SQL value expression AST.
#[derive(Clone, Debug)]
#[must_use]
pub enum ValueExpr {
    /// Field reference.
    Field {
        /// Field metadata.
        meta: Meta,
        /// Optional qualifier or source alias.
        qualifier: Option<String>,
    },
    /// `EXCLUDED.field` reference for upserts.
    Excluded(Meta),
    /// Bind parameter.
    Param(Param),
    /// SQL keyword expression such as `CURRENT_DATE`.
    Keyword(&'static str),
    /// Function call.
    Function {
        /// Function name.
        name: &'static str,
        /// Function arguments.
        args: Vec<ValueExpr>,
    },
    /// Aggregate function call.
    Aggregate {
        /// Aggregate function name.
        name: &'static str,
        /// Aggregate arguments.
        args: Vec<ValueExpr>,
        /// Whether to render `DISTINCT`.
        distinct: bool,
        /// Aggregate-local order by.
        order_by: Vec<OrderItem>,
        /// Aggregate `FILTER`.
        filter: Option<Box<BoolExpr>>,
    },
    /// Ordered-set aggregate function call.
    OrderedSetAggregate {
        /// Aggregate function name.
        name: &'static str,
        /// Aggregate arguments.
        args: Vec<ValueExpr>,
        /// `WITHIN GROUP` ordering.
        within_group: Vec<OrderItem>,
        /// Aggregate `FILTER`.
        filter: Option<Box<BoolExpr>>,
    },
    /// SQL `CASE` expression.
    Case {
        /// Ordered `WHEN` branches.
        branches: Vec<(BoolExpr, ValueExpr)>,
        /// Optional `ELSE` expression.
        else_: Option<Box<ValueExpr>>,
    },
    /// SQL cast.
    Cast {
        /// Casted expression.
        expr: Box<ValueExpr>,
        /// Postgres type name.
        pg: &'static str,
    },
    /// Binary value expression.
    Binary {
        /// Left expression.
        left: Box<ValueExpr>,
        /// Operator.
        op: ValueOp,
        /// Right expression.
        right: Box<ValueExpr>,
    },
    /// Array or JSON subscript.
    Subscript {
        /// Indexed expression.
        expr: Box<ValueExpr>,
        /// Index expression.
        index: Box<ValueExpr>,
    },
    /// Array slice.
    Slice {
        /// Sliced expression.
        expr: Box<ValueExpr>,
        /// Optional start bound.
        start: Option<Box<ValueExpr>>,
        /// Optional end bound.
        end: Option<Box<ValueExpr>>,
    },
    /// SQL array constructor.
    Array(Vec<ValueExpr>),
    /// SQL row constructor.
    Row(Vec<ValueExpr>),
    /// SQL `EXTRACT(field FROM expr)`.
    Extract {
        /// Extracted field name.
        field: &'static str,
        /// Source expression.
        expr: Box<ValueExpr>,
    },
    /// Window function call with `OVER`.
    Window {
        /// Window function name.
        function: WindowFunction,
        /// Window function arguments.
        args: Vec<ValueExpr>,
        /// Window specification.
        spec: WindowSpec,
    },
    /// Server-owned raw value expression.
    Raw {
        /// Raw SQL using rqb `?` placeholders.
        sql: String,
        /// Bind parameters for the raw SQL.
        params: Vec<Param>,
    },
    /// Scalar subquery expression.
    Subquery(Box<crate::Stmt>),
}
