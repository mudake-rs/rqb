use crate::typed::Param;

use super::{BoolExpr, ValueExpr};

impl BoolExpr {
    pub fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Constant(_) => {}
            Self::Compare { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::IsNull { expr, .. } => expr.collect_params(params),
            Self::InList { expr, values, .. } => {
                expr.collect_params(params);
                for value in values {
                    value.collect_params(params);
                }
            }
            Self::InSubquery { expr, query, .. } => {
                expr.collect_params(params);
                query.collect_params(params);
            }
            Self::Between {
                expr, low, high, ..
            } => {
                expr.collect_params(params);
                low.collect_params(params);
                high.collect_params(params);
            }
            Self::Like { expr, pattern, .. } => {
                expr.collect_params(params);
                pattern.collect_params(params);
            }
            Self::Regex { expr, pattern, .. } => {
                expr.collect_params(params);
                pattern.collect_params(params);
            }
            Self::Infix { left, right, .. } => {
                left.collect_params(params);
                right.collect_params(params);
            }
            Self::Any { value, array, .. } => {
                value.collect_params(params);
                array.collect_params(params);
            }
            Self::ArrayIsEmpty { expr, .. } => expr.collect_params(params),
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
    pub fn collect_params(&self, params: &mut Vec<Param>) {
        match self {
            Self::Param(param) => params.push(param.clone()),
            Self::Function { args, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
            }
            Self::Aggregate {
                args,
                order_by,
                filter,
                ..
            } => {
                for arg in args {
                    arg.collect_params(params);
                }
                for item in order_by {
                    item.collect_params(params);
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
            Self::Window { args, spec, .. } => {
                for arg in args {
                    arg.collect_params(params);
                }
                for expr in &spec.partition_by {
                    expr.collect_params(params);
                }
                for item in &spec.order_by {
                    item.collect_params(params);
                }
            }
            Self::Raw {
                params: raw_params, ..
            } => params.extend(raw_params.iter().cloned()),
            Self::Subquery(stmt) => stmt.collect_params(params),
            Self::Field { .. } | Self::Excluded(_) => {}
        }
    }
}
