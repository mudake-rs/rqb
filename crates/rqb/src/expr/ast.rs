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
#[doc(hidden)]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[non_exhaustive]
    UnboundedPreceding,
    /// `n PRECEDING`.
    #[non_exhaustive]
    Preceding(Box<ValueExpr>),
    /// `CURRENT ROW`.
    #[non_exhaustive]
    CurrentRow,
    /// `n FOLLOWING`.
    #[non_exhaustive]
    Following(Box<ValueExpr>),
    /// `UNBOUNDED FOLLOWING`.
    #[non_exhaustive]
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
#[non_exhaustive]
pub struct WindowFrame {
    /// Frame unit.
    pub(crate) kind: WindowFrameKind,
    /// Start boundary.
    pub(crate) start: FrameBound,
    /// Optional end boundary for `BETWEEN`.
    pub(crate) end: Option<FrameBound>,
    /// Optional exclusion clause.
    pub(crate) exclude: Option<FrameExclude>,
}

/// Window specification used by `OVER (...)`.
#[derive(Clone, Debug, Default)]
#[must_use]
#[non_exhaustive]
pub struct WindowSpec {
    /// Partition expressions.
    pub(crate) partition_by: Vec<ValueExpr>,
    /// Ordering expressions.
    pub(crate) order_by: Vec<OrderItem>,
    /// Optional frame.
    pub(crate) frame: Option<Box<WindowFrame>>,
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
#[non_exhaustive]
pub enum BoolExpr {
    /// Boolean constant.
    #[non_exhaustive]
    Constant(bool),
    /// Binary comparison.
    #[non_exhaustive]
    Compare {
        /// Left side.
        left: ValueExpr,
        /// Comparison operator.
        op: BoolOp,
        /// Right side.
        right: ValueExpr,
    },
    /// `IS NULL` / `IS NOT NULL`.
    #[non_exhaustive]
    IsNull {
        /// Tested expression.
        expr: ValueExpr,
        /// Whether to render `IS NOT NULL`.
        negated: bool,
    },
    /// `IS TRUE` / `IS NOT TRUE` and related boolean tests.
    #[non_exhaustive]
    IsBoolean {
        /// Tested expression.
        expr: ValueExpr,
        /// Boolean test kind.
        test: BooleanTest,
        /// Whether to negate the test.
        negated: bool,
    },
    /// `IN (...)` / `NOT IN (...)`.
    #[non_exhaustive]
    InList {
        /// Tested expression.
        expr: ValueExpr,
        /// List values.
        values: Vec<ValueExpr>,
        /// Whether to render `NOT IN`.
        negated: bool,
    },
    /// `IN (subquery)` / `NOT IN (subquery)`.
    #[non_exhaustive]
    InSubquery {
        /// Tested expression.
        expr: ValueExpr,
        /// Subquery statement.
        query: Box<crate::Stmt>,
        /// Whether to render `NOT IN`.
        negated: bool,
    },
    /// `BETWEEN` / `NOT BETWEEN`.
    #[non_exhaustive]
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
    #[non_exhaustive]
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
    #[non_exhaustive]
    SimilarTo {
        /// Tested expression.
        expr: ValueExpr,
        /// Pattern expression.
        pattern: ValueExpr,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// PostgreSQL regex predicate.
    #[non_exhaustive]
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
    #[non_exhaustive]
    Infix {
        /// Left expression.
        left: ValueExpr,
        /// SQL operator token.
        op: &'static str,
        /// Right expression.
        right: ValueExpr,
        /// Whether to wrap with `NOT`.
        negated: bool,
        /// Whether a named DSL helper requires metadata/type checks.
        checked: bool,
    },
    /// `value = ANY(array)` / negated form.
    #[non_exhaustive]
    Any {
        /// Value expression.
        value: ValueExpr,
        /// Array expression.
        array: ValueExpr,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// Array-empty predicate.
    #[non_exhaustive]
    ArrayIsEmpty {
        /// Array expression.
        expr: ValueExpr,
        /// Whether to negate the predicate.
        negated: bool,
    },
    /// Logical conjunction.
    #[non_exhaustive]
    And(Vec<BoolExpr>),
    /// Logical disjunction.
    #[non_exhaustive]
    Or(Vec<BoolExpr>),
    /// Logical negation.
    #[non_exhaustive]
    Not(Box<BoolExpr>),
    /// `EXISTS (subquery)`.
    #[non_exhaustive]
    Exists(Box<crate::Stmt>),
    /// Server-owned raw predicate.
    #[non_exhaustive]
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
#[non_exhaustive]
pub enum ValueExpr {
    /// Field reference.
    #[non_exhaustive]
    Field {
        /// Field metadata.
        meta: Meta,
        /// Optional qualifier or source alias.
        qualifier: Option<String>,
    },
    /// `EXCLUDED.field` reference for upserts.
    #[non_exhaustive]
    Excluded(Meta),
    /// Bind parameter.
    #[non_exhaustive]
    Param(Param),
    /// SQL `NULL` literal.
    #[non_exhaustive]
    Null,
    /// Server-owned static SQL string literal.
    #[non_exhaustive]
    SqlLiteral(&'static str),
    /// SQL keyword expression such as `CURRENT_DATE`.
    #[non_exhaustive]
    Keyword(&'static str),
    /// Function call.
    #[non_exhaustive]
    Function {
        /// Function name.
        name: &'static str,
        /// Function arguments.
        args: Vec<ValueExpr>,
    },
    /// Aggregate function call.
    #[non_exhaustive]
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
        /// Optional window specification for aggregate window functions.
        over: Option<Box<WindowSpec>>,
    },
    /// Ordered-set aggregate function call.
    #[non_exhaustive]
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
    #[non_exhaustive]
    Case {
        /// Ordered `WHEN` branches.
        branches: Vec<(BoolExpr, ValueExpr)>,
        /// Optional `ELSE` expression.
        else_: Option<Box<ValueExpr>>,
    },
    /// SQL cast.
    #[non_exhaustive]
    Cast {
        /// Casted expression.
        expr: Box<ValueExpr>,
        /// Postgres type name.
        pg: &'static str,
    },
    /// Binary value expression.
    #[non_exhaustive]
    Binary {
        /// Left expression.
        left: Box<ValueExpr>,
        /// Operator.
        op: ValueOp,
        /// Right expression.
        right: Box<ValueExpr>,
    },
    /// Array or JSON subscript.
    #[non_exhaustive]
    Subscript {
        /// Indexed expression.
        expr: Box<ValueExpr>,
        /// Index expression.
        index: Box<ValueExpr>,
    },
    /// Array slice.
    #[non_exhaustive]
    Slice {
        /// Sliced expression.
        expr: Box<ValueExpr>,
        /// Optional start bound.
        start: Option<Box<ValueExpr>>,
        /// Optional end bound.
        end: Option<Box<ValueExpr>>,
    },
    /// SQL array constructor.
    #[non_exhaustive]
    Array(Vec<ValueExpr>),
    /// SQL row constructor.
    #[non_exhaustive]
    Row(Vec<ValueExpr>),
    /// SQL `EXTRACT(field FROM expr)`.
    #[non_exhaustive]
    Extract {
        /// Extracted field name.
        field: &'static str,
        /// Source expression.
        expr: Box<ValueExpr>,
    },
    /// Window function call with `OVER`.
    #[non_exhaustive]
    Window {
        /// Window function name.
        function: WindowFunction,
        /// Window function arguments.
        args: Vec<ValueExpr>,
        /// Window specification.
        spec: WindowSpec,
    },
    /// Server-owned raw value expression.
    #[non_exhaustive]
    Raw {
        /// Raw SQL using rqb `?` placeholders.
        sql: String,
        /// Bind parameters for the raw SQL.
        params: Vec<Param>,
    },
    /// Scalar subquery expression.
    #[non_exhaustive]
    Subquery(Box<crate::Stmt>),
    /// Invalid aggregate-local modifier use retained for validation.
    #[non_exhaustive]
    InvalidAggregateModifier {
        /// Expression the modifier was applied to.
        expr: Box<ValueExpr>,
        /// Modifier method name.
        modifier: &'static str,
    },
}

impl BoolExpr {
    #[inline]
    pub(crate) fn compare(left: ValueExpr, op: BoolOp, right: ValueExpr) -> Self {
        Self::Compare { left, op, right }
    }

    #[inline]
    pub(crate) fn is_null_expr(expr: ValueExpr, negated: bool) -> Self {
        Self::IsNull { expr, negated }
    }

    #[inline]
    pub(crate) fn is_boolean(expr: ValueExpr, test: BooleanTest, negated: bool) -> Self {
        Self::IsBoolean {
            expr,
            test,
            negated,
        }
    }

    #[inline]
    pub(crate) fn in_list(expr: ValueExpr, values: Vec<ValueExpr>, negated: bool) -> Self {
        Self::InList {
            expr,
            values,
            negated,
        }
    }

    #[inline]
    pub(crate) fn in_subquery(expr: ValueExpr, query: Box<crate::Stmt>, negated: bool) -> Self {
        Self::InSubquery {
            expr,
            query,
            negated,
        }
    }

    #[inline]
    pub(crate) fn between(expr: ValueExpr, low: ValueExpr, high: ValueExpr, negated: bool) -> Self {
        Self::Between {
            expr,
            low,
            high,
            negated,
        }
    }

    #[inline]
    pub(crate) fn like(
        expr: ValueExpr,
        pattern: ValueExpr,
        case_insensitive: bool,
        negated: bool,
        escape: bool,
    ) -> Self {
        Self::Like {
            expr,
            pattern,
            case_insensitive,
            negated,
            escape,
        }
    }

    #[inline]
    pub(crate) fn similar_to(expr: ValueExpr, pattern: ValueExpr, negated: bool) -> Self {
        Self::SimilarTo {
            expr,
            pattern,
            negated,
        }
    }

    #[inline]
    pub(crate) fn regex(
        expr: ValueExpr,
        pattern: ValueExpr,
        case_insensitive: bool,
        negated: bool,
    ) -> Self {
        Self::Regex {
            expr,
            pattern,
            case_insensitive,
            negated,
        }
    }

    #[inline]
    pub(crate) fn infix(
        left: ValueExpr,
        op: &'static str,
        right: ValueExpr,
        negated: bool,
    ) -> Self {
        Self::Infix {
            left,
            op,
            right,
            negated,
            checked: true,
        }
    }

    #[inline]
    pub(crate) fn any(value: ValueExpr, array: ValueExpr, negated: bool) -> Self {
        Self::Any {
            value,
            array,
            negated,
        }
    }

    #[inline]
    pub(crate) fn array_is_empty(expr: ValueExpr, negated: bool) -> Self {
        Self::ArrayIsEmpty { expr, negated }
    }

    #[inline]
    pub(crate) fn raw(sql: impl Into<String>, params: impl Into<Vec<Param>>) -> Self {
        Self::Raw {
            sql: sql.into(),
            params: params.into(),
        }
    }
}

impl ValueExpr {
    #[inline]
    pub(crate) fn field(meta: Meta, qualifier: Option<String>) -> Self {
        Self::Field { meta, qualifier }
    }

    #[inline]
    pub(crate) fn function(name: &'static str, args: Vec<ValueExpr>) -> Self {
        Self::Function { name, args }
    }

    #[inline]
    pub(crate) fn aggregate(
        name: &'static str,
        args: Vec<ValueExpr>,
        distinct: bool,
        order_by: Vec<OrderItem>,
        filter: Option<Box<BoolExpr>>,
        over: Option<Box<WindowSpec>>,
    ) -> Self {
        Self::Aggregate {
            name,
            args,
            distinct,
            order_by,
            filter,
            over,
        }
    }

    #[inline]
    pub(crate) fn ordered_set_aggregate(
        name: &'static str,
        args: Vec<ValueExpr>,
        within_group: Vec<OrderItem>,
        filter: Option<Box<BoolExpr>>,
    ) -> Self {
        Self::OrderedSetAggregate {
            name,
            args,
            within_group,
            filter,
        }
    }

    #[inline]
    pub(crate) fn case(
        branches: Vec<(BoolExpr, ValueExpr)>,
        else_: Option<Box<ValueExpr>>,
    ) -> Self {
        Self::Case { branches, else_ }
    }

    #[inline]
    pub(crate) fn cast_expr(expr: ValueExpr, pg: &'static str) -> Self {
        Self::Cast {
            expr: Box::new(expr),
            pg,
        }
    }

    #[inline]
    pub(crate) fn binary(left: ValueExpr, op: ValueOp, right: ValueExpr) -> Self {
        Self::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    #[inline]
    pub(crate) fn subscript(expr: ValueExpr, index: ValueExpr) -> Self {
        Self::Subscript {
            expr: Box::new(expr),
            index: Box::new(index),
        }
    }

    #[inline]
    pub(crate) fn slice(expr: ValueExpr, start: Option<ValueExpr>, end: Option<ValueExpr>) -> Self {
        Self::Slice {
            expr: Box::new(expr),
            start: start.map(Box::new),
            end: end.map(Box::new),
        }
    }

    #[inline]
    pub(crate) fn extract(field: &'static str, expr: ValueExpr) -> Self {
        Self::Extract {
            field,
            expr: Box::new(expr),
        }
    }

    #[inline]
    pub(crate) fn window(function: WindowFunction, args: Vec<ValueExpr>, spec: WindowSpec) -> Self {
        Self::Window {
            function,
            args,
            spec,
        }
    }

    #[inline]
    pub(crate) fn raw(sql: impl Into<String>, params: impl Into<Vec<Param>>) -> Self {
        Self::Raw {
            sql: sql.into(),
            params: params.into(),
        }
    }

    #[inline]
    pub(crate) fn invalid_aggregate_modifier(expr: ValueExpr, modifier: &'static str) -> Self {
        Self::InvalidAggregateModifier {
            expr: Box::new(expr),
            modifier,
        }
    }
}
