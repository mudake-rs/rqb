use super::BoolExpr;

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
}
