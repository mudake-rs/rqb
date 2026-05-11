use super::{BoolExpr, BooleanTest, Field, FieldRef, ValueExpr};

/// Builds a SQL `TRUE` predicate.
pub const fn true_() -> BoolExpr {
    BoolExpr::Constant(true)
}

/// Builds a SQL `FALSE` predicate.
pub const fn false_() -> BoolExpr {
    BoolExpr::Constant(false)
}

/// Builds a logical `AND` group.
pub fn and(exprs: impl IntoIterator<Item = BoolExpr>) -> BoolExpr {
    BoolExpr::and(exprs)
}

/// Builds a logical `OR` group.
pub fn or(exprs: impl IntoIterator<Item = BoolExpr>) -> BoolExpr {
    BoolExpr::or(exprs)
}

/// Builds an `EXISTS (...)` predicate from a server-owned subquery.
pub fn exists(stmt: impl Into<crate::Stmt>) -> BoolExpr {
    BoolExpr::Exists(Box::new(stmt.into()))
}

/// Negates a predicate.
pub fn not(expr: BoolExpr) -> BoolExpr {
    BoolExpr::negate(expr)
}

impl BoolExpr {
    /// Builds a logical `AND` group.
    pub fn and(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::And(exprs.into_iter().collect())
    }

    /// Builds a logical `OR` group.
    pub fn or(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::Or(exprs.into_iter().collect())
    }

    /// Builds a logical `NOT`.
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

    pub(crate) fn or_pair(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::Or(mut left), Self::Or(right)) if !left.is_empty() && !right.is_empty() => {
                left.extend(right);
                Self::Or(left)
            }
            (Self::Or(mut left), right) if !left.is_empty() => {
                left.push(right);
                Self::Or(left)
            }
            (left, Self::Or(mut right)) if !right.is_empty() => {
                right.insert(0, left);
                Self::Or(right)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }

    pub(crate) fn or_option(current: Option<Self>, next: Self) -> Self {
        match current {
            Some(existing) => Self::or_pair(existing, next),
            None => next,
        }
    }
}

impl ValueExpr {
    /// Builds `expr IS TRUE`.
    pub fn is_true(self) -> BoolExpr {
        self.boolean_test(BooleanTest::True, false)
    }

    /// Builds `expr IS NOT TRUE`.
    pub fn is_not_true(self) -> BoolExpr {
        self.boolean_test(BooleanTest::True, true)
    }

    /// Builds `expr IS FALSE`.
    pub fn is_false(self) -> BoolExpr {
        self.boolean_test(BooleanTest::False, false)
    }

    /// Builds `expr IS NOT FALSE`.
    pub fn is_not_false(self) -> BoolExpr {
        self.boolean_test(BooleanTest::False, true)
    }

    /// Builds `expr IS UNKNOWN`.
    pub fn is_unknown(self) -> BoolExpr {
        self.boolean_test(BooleanTest::Unknown, false)
    }

    /// Builds `expr IS NOT UNKNOWN`.
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
    /// Builds `field IS TRUE`.
    pub fn is_true(self) -> BoolExpr {
        self.expr().is_true()
    }

    /// Builds `field IS NOT TRUE`.
    pub fn is_not_true(self) -> BoolExpr {
        self.expr().is_not_true()
    }

    /// Builds `field IS FALSE`.
    pub fn is_false(self) -> BoolExpr {
        self.expr().is_false()
    }

    /// Builds `field IS NOT FALSE`.
    pub fn is_not_false(self) -> BoolExpr {
        self.expr().is_not_false()
    }

    /// Builds `field IS UNKNOWN`.
    pub fn is_unknown(self) -> BoolExpr {
        self.expr().is_unknown()
    }

    /// Builds `field IS NOT UNKNOWN`.
    pub fn is_not_unknown(self) -> BoolExpr {
        self.expr().is_not_unknown()
    }
}

impl FieldRef<bool> {
    /// Builds `field IS TRUE`.
    pub fn is_true(self) -> BoolExpr {
        self.expr().is_true()
    }

    /// Builds `field IS NOT TRUE`.
    pub fn is_not_true(self) -> BoolExpr {
        self.expr().is_not_true()
    }

    /// Builds `field IS FALSE`.
    pub fn is_false(self) -> BoolExpr {
        self.expr().is_false()
    }

    /// Builds `field IS NOT FALSE`.
    pub fn is_not_false(self) -> BoolExpr {
        self.expr().is_not_false()
    }

    /// Builds `field IS UNKNOWN`.
    pub fn is_unknown(self) -> BoolExpr {
        self.expr().is_unknown()
    }

    /// Builds `field IS NOT UNKNOWN`.
    pub fn is_not_unknown(self) -> BoolExpr {
        self.expr().is_not_unknown()
    }
}
