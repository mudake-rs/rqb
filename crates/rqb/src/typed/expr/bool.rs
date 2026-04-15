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

    pub(crate) fn and_pair(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::And(mut left), Self::And(right)) if !left.is_empty() && !right.is_empty() => {
                left.extend(right);
                Self::And(left)
            }
            (Self::And(mut left), right) if !left.is_empty() => {
                left.push(right);
                Self::And(left)
            }
            (left, Self::And(mut right)) if !right.is_empty() => {
                right.insert(0, left);
                Self::And(right)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    pub(crate) fn and_option(current: Option<Self>, next: Self) -> Self {
        match current {
            Some(existing) => Self::and_pair(existing, next),
            None => next,
        }
    }
}
