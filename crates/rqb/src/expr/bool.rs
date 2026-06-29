use super::{BoolExpr, BooleanTest, Field, FieldRef, ValueExpr};

/// Builds a SQL `TRUE` predicate.
#[inline]
pub const fn true_() -> BoolExpr {
    BoolExpr::Constant(true)
}

/// Builds a SQL `FALSE` predicate.
#[inline]
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
#[inline]
pub fn not(expr: BoolExpr) -> BoolExpr {
    BoolExpr::negate(expr)
}

impl BoolExpr {
    /// Builds a logical `AND` group.
    pub fn and(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::And(flatten_and(exprs))
    }

    /// Builds a logical `OR` group.
    pub fn or(exprs: impl IntoIterator<Item = BoolExpr>) -> Self {
        Self::Or(flatten_or(exprs))
    }

    /// Builds a logical `NOT`.
    #[inline]
    pub fn negate(expr: BoolExpr) -> Self {
        Self::Not(Box::new(expr))
    }

    pub(crate) fn and_pair(left: Self, right: Self) -> Self {
        match (left, right) {
            (BoolExpr::And(mut left), BoolExpr::And(right))
                if !left.is_empty() && !right.is_empty() =>
            {
                left.extend(right);
                Self::And(left)
            }
            (BoolExpr::And(mut left), right) if !left.is_empty() => {
                left.push(right);
                Self::And(left)
            }
            (left, BoolExpr::And(right)) if !right.is_empty() => {
                let mut exprs = Vec::with_capacity(right.len() + 1);
                exprs.push(left);
                exprs.extend(right);
                Self::And(exprs)
            }
            (left, right) => Self::And(vec![left, right]),
        }
    }

    #[inline]
    pub(crate) fn and_option(current: Option<Self>, next: Self) -> Self {
        match current {
            Some(existing) => Self::and_pair(existing, next),
            None => next,
        }
    }

    pub(crate) fn or_pair(left: Self, right: Self) -> Self {
        match (left, right) {
            (BoolExpr::Or(mut left), BoolExpr::Or(right))
                if !left.is_empty() && !right.is_empty() =>
            {
                left.extend(right);
                Self::Or(left)
            }
            (BoolExpr::Or(mut left), right) if !left.is_empty() => {
                left.push(right);
                Self::Or(left)
            }
            (left, BoolExpr::Or(right)) if !right.is_empty() => {
                let mut exprs = Vec::with_capacity(right.len() + 1);
                exprs.push(left);
                exprs.extend(right);
                Self::Or(exprs)
            }
            (left, right) => Self::Or(vec![left, right]),
        }
    }

    #[inline]
    pub(crate) fn or_option(current: Option<Self>, next: Self) -> Self {
        match current {
            Some(existing) => Self::or_pair(existing, next),
            None => next,
        }
    }
}

fn flatten_and(exprs: impl IntoIterator<Item = BoolExpr>) -> Vec<BoolExpr> {
    let exprs = exprs.into_iter();
    let mut flattened = Vec::with_capacity(exprs.size_hint().0);
    for expr in exprs {
        match expr {
            BoolExpr::And(inner) if !inner.is_empty() => flattened.extend(inner),
            other => flattened.push(other),
        }
    }
    flattened
}

fn flatten_or(exprs: impl IntoIterator<Item = BoolExpr>) -> Vec<BoolExpr> {
    let exprs = exprs.into_iter();
    let mut flattened = Vec::with_capacity(exprs.size_hint().0);
    for expr in exprs {
        match expr {
            BoolExpr::Or(inner) if !inner.is_empty() => flattened.extend(inner),
            other => flattened.push(other),
        }
    }
    flattened
}

impl ValueExpr {
    /// Builds `expr IS TRUE`.
    #[inline]
    pub fn is_true(self) -> BoolExpr {
        self.boolean_test(BooleanTest::True, false)
    }

    /// Builds `expr IS NOT TRUE`.
    #[inline]
    pub fn is_not_true(self) -> BoolExpr {
        self.boolean_test(BooleanTest::True, true)
    }

    /// Builds `expr IS FALSE`.
    #[inline]
    pub fn is_false(self) -> BoolExpr {
        self.boolean_test(BooleanTest::False, false)
    }

    /// Builds `expr IS NOT FALSE`.
    #[inline]
    pub fn is_not_false(self) -> BoolExpr {
        self.boolean_test(BooleanTest::False, true)
    }

    /// Builds `expr IS UNKNOWN`.
    #[inline]
    pub fn is_unknown(self) -> BoolExpr {
        self.boolean_test(BooleanTest::Unknown, false)
    }

    /// Builds `expr IS NOT UNKNOWN`.
    #[inline]
    pub fn is_not_unknown(self) -> BoolExpr {
        self.boolean_test(BooleanTest::Unknown, true)
    }

    fn boolean_test(self, test: BooleanTest, negated: bool) -> BoolExpr {
        BoolExpr::is_boolean(self, test, negated)
    }
}

macro_rules! boolean_field_tests {
    () => {
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
    };
}

impl Field<bool> {
    boolean_field_tests!();
}

impl FieldRef<bool> {
    boolean_field_tests!();
}
