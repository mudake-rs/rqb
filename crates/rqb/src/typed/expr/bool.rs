use super::{BoolExpr, BooleanTest, Field, FieldRef, ValueExpr};

pub fn all(exprs: impl IntoIterator<Item = BoolExpr>) -> BoolExpr {
    BoolExpr::and(exprs)
}

pub fn any(exprs: impl IntoIterator<Item = BoolExpr>) -> BoolExpr {
    BoolExpr::or(exprs)
}

/// Builds an `EXISTS (...)` predicate from a server-owned subquery.
pub fn exists(stmt: impl Into<crate::typed::Stmt>) -> BoolExpr {
    BoolExpr::Exists(Box::new(stmt.into()))
}

/// Negates a predicate.
pub fn not(expr: BoolExpr) -> BoolExpr {
    BoolExpr::negate(expr)
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

impl ValueExpr {
    pub fn is_true(self) -> BoolExpr {
        self.boolean_test(BooleanTest::True, false)
    }

    pub fn is_not_true(self) -> BoolExpr {
        self.boolean_test(BooleanTest::True, true)
    }

    pub fn is_false(self) -> BoolExpr {
        self.boolean_test(BooleanTest::False, false)
    }

    pub fn is_not_false(self) -> BoolExpr {
        self.boolean_test(BooleanTest::False, true)
    }

    pub fn is_unknown(self) -> BoolExpr {
        self.boolean_test(BooleanTest::Unknown, false)
    }

    pub fn is_not_unknown(self) -> BoolExpr {
        self.boolean_test(BooleanTest::Unknown, true)
    }

    fn boolean_test(self, test: BooleanTest, negated: bool) -> BoolExpr {
        BoolExpr::IsBoolean {
            expr: self,
            test,
            negated,
        }
    }
}

impl Field<bool> {
    pub fn is_true(self) -> BoolExpr {
        self.expr().is_true()
    }

    pub fn is_not_true(self) -> BoolExpr {
        self.expr().is_not_true()
    }

    pub fn is_false(self) -> BoolExpr {
        self.expr().is_false()
    }

    pub fn is_not_false(self) -> BoolExpr {
        self.expr().is_not_false()
    }

    pub fn is_unknown(self) -> BoolExpr {
        self.expr().is_unknown()
    }

    pub fn is_not_unknown(self) -> BoolExpr {
        self.expr().is_not_unknown()
    }
}

impl FieldRef<bool> {
    pub fn is_true(self) -> BoolExpr {
        self.expr().is_true()
    }

    pub fn is_not_true(self) -> BoolExpr {
        self.expr().is_not_true()
    }

    pub fn is_false(self) -> BoolExpr {
        self.expr().is_false()
    }

    pub fn is_not_false(self) -> BoolExpr {
        self.expr().is_not_false()
    }

    pub fn is_unknown(self) -> BoolExpr {
        self.expr().is_unknown()
    }

    pub fn is_not_unknown(self) -> BoolExpr {
        self.expr().is_not_unknown()
    }
}
