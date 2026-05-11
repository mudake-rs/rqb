use sqlx::postgres::PgHasArrayType;

use crate::{BindValue, Param};

use super::{BoolExpr, Field, FieldRef, ValueExpr};

impl<T> Field<Vec<T>> {
    /// Builds an array overlap predicate (`&&`) from a SQL array expression.
    pub fn overlaps_expr(self, values: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().overlaps_expr(values)
    }

    /// Builds an array contains-all predicate (`@>`) from a SQL array expression.
    pub fn contains_expr(self, values: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().contains_expr(values)
    }

    /// Builds an array contained-by predicate (`<@`) from a SQL array expression.
    pub fn contained_by_expr(self, values: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().contained_by_expr(values)
    }

    /// Builds `value = ANY(array)` from a SQL value expression.
    pub fn has_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().has_expr(value)
    }

    /// Builds the negated `ANY` membership predicate from a SQL value expression.
    pub fn not_has_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.reference().not_has_expr(value)
    }
}

impl<T> Field<Vec<T>>
where
    T: BindValue + PgHasArrayType,
{
    /// Builds an array overlap predicate (`&&`).
    pub fn overlaps(self, values: Vec<T>) -> BoolExpr {
        self.reference().overlaps(values)
    }

    /// Builds an array contains-all predicate (`@>`).
    pub fn contains(self, values: Vec<T>) -> BoolExpr {
        self.reference().contains(values)
    }

    /// Builds an array contained-by predicate (`<@`).
    pub fn contained_by(self, values: Vec<T>) -> BoolExpr {
        self.reference().contained_by(values)
    }

    /// Builds `value = ANY(array)`.
    pub fn has(self, value: T) -> BoolExpr {
        self.reference().has(value)
    }

    /// Builds the negated `ANY` membership predicate.
    pub fn not_has(self, value: T) -> BoolExpr {
        self.reference().not_has(value)
    }

    /// Builds an empty-array predicate.
    pub fn is_empty(self) -> BoolExpr {
        self.reference().is_empty()
    }

    /// Builds a non-empty-array predicate.
    pub fn is_not_empty(self) -> BoolExpr {
        self.reference().is_not_empty()
    }

    /// Builds an array subscript expression.
    pub fn element(self, index: i32) -> ValueExpr {
        self.reference().element(index)
    }

    /// Builds an array slice expression.
    pub fn slice(self, start: Option<i32>, end: Option<i32>) -> ValueExpr {
        self.reference().slice(start, end)
    }
}

impl<T> FieldRef<Vec<T>> {
    /// Builds an array overlap predicate (`&&`) from a SQL array expression.
    pub fn overlaps_expr(self, values: impl Into<ValueExpr>) -> BoolExpr {
        self.array_expr_infix("&&", values, false)
    }

    /// Builds an array contains-all predicate (`@>`) from a SQL array expression.
    pub fn contains_expr(self, values: impl Into<ValueExpr>) -> BoolExpr {
        self.array_expr_infix("@>", values, false)
    }

    /// Builds an array contained-by predicate (`<@`) from a SQL array expression.
    pub fn contained_by_expr(self, values: impl Into<ValueExpr>) -> BoolExpr {
        self.array_expr_infix("<@", values, false)
    }

    /// Builds `value = ANY(array)` from a SQL value expression.
    pub fn has_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.any_expr_predicate(value, false)
    }

    /// Builds the negated `ANY` membership predicate from a SQL value expression.
    pub fn not_has_expr(self, value: impl Into<ValueExpr>) -> BoolExpr {
        self.any_expr_predicate(value, true)
    }

    fn array_expr_infix(
        self,
        op: &'static str,
        values: impl Into<ValueExpr>,
        negated: bool,
    ) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: values.into(),
            negated,
        }
    }

    fn any_expr_predicate(self, value: impl Into<ValueExpr>, negated: bool) -> BoolExpr {
        BoolExpr::Any {
            value: value.into(),
            array: self.expr(),
            negated,
        }
    }
}

impl<T> FieldRef<Vec<T>>
where
    T: BindValue + PgHasArrayType,
{
    /// Builds an array overlap predicate (`&&`).
    pub fn overlaps(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("&&", values, false)
    }

    /// Builds an array contains-all predicate (`@>`).
    pub fn contains(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("@>", values, false)
    }

    /// Builds an array contained-by predicate (`<@`).
    pub fn contained_by(self, values: Vec<T>) -> BoolExpr {
        self.array_infix("<@", values, false)
    }

    /// Builds `value = ANY(array)`.
    pub fn has(self, value: T) -> BoolExpr {
        self.any_predicate(value, false)
    }

    /// Builds the negated `ANY` membership predicate.
    pub fn not_has(self, value: T) -> BoolExpr {
        self.any_predicate(value, true)
    }

    /// Builds an empty-array predicate.
    pub fn is_empty(self) -> BoolExpr {
        BoolExpr::ArrayIsEmpty {
            expr: self.expr(),
            negated: false,
        }
    }

    /// Builds a non-empty-array predicate.
    pub fn is_not_empty(self) -> BoolExpr {
        BoolExpr::ArrayIsEmpty {
            expr: self.expr(),
            negated: true,
        }
    }

    /// Builds an array subscript expression.
    pub fn element(self, index: i32) -> ValueExpr {
        ValueExpr::Subscript {
            expr: Box::new(self.expr()),
            index: Box::new(ValueExpr::Param(Param::typed(index))),
        }
    }

    /// Builds an array slice expression.
    pub fn slice(self, start: Option<i32>, end: Option<i32>) -> ValueExpr {
        ValueExpr::Slice {
            expr: Box::new(self.expr()),
            start: start.map(|value| Box::new(ValueExpr::Param(Param::typed(value)))),
            end: end.map(|value| Box::new(ValueExpr::Param(Param::typed(value)))),
        }
    }

    fn array_infix(self, op: &'static str, values: Vec<T>, negated: bool) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(values)),
            negated,
        }
    }

    fn any_predicate(self, value: T, negated: bool) -> BoolExpr {
        BoolExpr::Any {
            value: ValueExpr::Param(Param::typed(value)),
            array: self.expr(),
            negated,
        }
    }
}

impl Field<serde_json::Value> {
    /// Builds a JSONB containment predicate (`@>`).
    pub fn contains(self, value: serde_json::Value) -> BoolExpr {
        self.reference().contains(value)
    }

    /// Builds a JSONB contained-by predicate (`<@`).
    pub fn contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.reference().contained_by(value)
    }

    /// Builds a JSONB key-exists predicate (`?`).
    pub fn has_key(self, key: impl Into<String>) -> BoolExpr {
        self.reference().has_key(key)
    }

    /// Builds a JSONB any-key-exists predicate (`?|`).
    pub fn has_any_keys(self, keys: Vec<String>) -> BoolExpr {
        self.reference().has_any_keys(keys)
    }

    /// Builds a JSONB all-keys-exist predicate (`?&`).
    pub fn has_all_keys(self, keys: Vec<String>) -> BoolExpr {
        self.reference().has_all_keys(keys)
    }

    /// Builds a JSON value access expression (`->`).
    pub fn get(self, key: impl Into<String>) -> ValueExpr {
        self.reference().get(key)
    }

    /// Builds a JSON text access expression (`->>`).
    pub fn get_text(self, key: impl Into<String>) -> ValueExpr {
        self.reference().get_text(key)
    }

    /// Builds a JSON path access expression (`#>`).
    pub fn path(self, path: Vec<String>) -> ValueExpr {
        self.reference().path(path)
    }

    /// Builds a JSON path text access expression (`#>>`).
    pub fn path_text(self, path: Vec<String>) -> ValueExpr {
        self.reference().path_text(path)
    }
}

impl<T> Field<sqlx::postgres::types::PgRange<T>>
where
    T: BindValue,
    sqlx::postgres::types::PgRange<T>: BindValue,
{
    /// Builds a range contains element predicate (`@>`).
    pub fn range_contains(self, value: T) -> BoolExpr {
        self.reference().range_contains(value)
    }

    /// Builds a range contains range predicate (`@>`).
    pub fn contains_range(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().contains_range(value)
    }

    /// Builds a range contained-by predicate (`<@`).
    pub fn contained_by(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().contained_by(value)
    }

    /// Builds a range overlap predicate (`&&`).
    pub fn overlaps(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().overlaps(value)
    }

    /// Builds a range adjacency predicate (`-|-`).
    pub fn adjacent_to(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().adjacent_to(value)
    }

    /// Builds a strictly-left range predicate (`<<`).
    pub fn strictly_left_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().strictly_left_of(value)
    }

    /// Builds a strictly-right range predicate (`>>`).
    pub fn strictly_right_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().strictly_right_of(value)
    }

    /// Builds a does-not-extend-right range predicate (`&<`).
    pub fn does_not_extend_right_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().does_not_extend_right_of(value)
    }

    /// Builds a does-not-extend-left range predicate (`&>`).
    pub fn does_not_extend_left_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.reference().does_not_extend_left_of(value)
    }
}

impl<T> FieldRef<sqlx::postgres::types::PgRange<T>>
where
    T: BindValue,
    sqlx::postgres::types::PgRange<T>: BindValue,
{
    /// Builds a range contains element predicate (`@>`).
    pub fn range_contains(self, value: T) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op: "@>",
            right: ValueExpr::Param(Param::typed(value)),
            negated: false,
        }
    }

    /// Builds a range contains range predicate (`@>`).
    pub fn contains_range(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("@>", value)
    }

    /// Builds a range contained-by predicate (`<@`).
    pub fn contained_by(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("<@", value)
    }

    /// Builds a range overlap predicate (`&&`).
    pub fn overlaps(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("&&", value)
    }

    /// Builds a range adjacency predicate (`-|-`).
    pub fn adjacent_to(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("-|-", value)
    }

    /// Builds a strictly-left range predicate (`<<`).
    pub fn strictly_left_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("<<", value)
    }

    /// Builds a strictly-right range predicate (`>>`).
    pub fn strictly_right_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix(">>", value)
    }

    /// Builds a does-not-extend-right range predicate (`&<`).
    pub fn does_not_extend_right_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("&<", value)
    }

    /// Builds a does-not-extend-left range predicate (`&>`).
    pub fn does_not_extend_left_of(self, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        self.range_infix("&>", value)
    }

    fn range_infix(self, op: &'static str, value: sqlx::postgres::types::PgRange<T>) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(Param::typed(value)),
            negated: false,
        }
    }
}

impl FieldRef<serde_json::Value> {
    /// Builds a JSONB containment predicate (`@>`).
    pub fn contains(self, value: serde_json::Value) -> BoolExpr {
        self.json_infix("@>", Param::typed(value))
    }

    /// Builds a JSONB contained-by predicate (`<@`).
    pub fn contained_by(self, value: serde_json::Value) -> BoolExpr {
        self.json_infix("<@", Param::typed(value))
    }

    /// Builds a JSONB key-exists predicate (`?`).
    pub fn has_key(self, key: impl Into<String>) -> BoolExpr {
        self.json_infix("?", Param::typed(key.into()))
    }

    /// Builds a JSONB any-key-exists predicate (`?|`).
    pub fn has_any_keys(self, keys: Vec<String>) -> BoolExpr {
        self.json_infix("?|", Param::typed(keys))
    }

    /// Builds a JSONB all-keys-exist predicate (`?&`).
    pub fn has_all_keys(self, keys: Vec<String>) -> BoolExpr {
        self.json_infix("?&", Param::typed(keys))
    }

    /// Builds a JSON value access expression (`->`).
    pub fn get(self, key: impl Into<String>) -> ValueExpr {
        self.json_value_infix("->", Param::typed(key.into()))
    }

    /// Builds a JSON text access expression (`->>`).
    pub fn get_text(self, key: impl Into<String>) -> ValueExpr {
        self.json_value_infix("->>", Param::typed(key.into()))
    }

    /// Builds a JSON path access expression (`#>`).
    pub fn path(self, path: Vec<String>) -> ValueExpr {
        self.json_value_infix("#>", Param::typed(path))
    }

    /// Builds a JSON path text access expression (`#>>`).
    pub fn path_text(self, path: Vec<String>) -> ValueExpr {
        self.json_value_infix("#>>", Param::typed(path))
    }

    fn json_infix(self, op: &'static str, param: Param) -> BoolExpr {
        BoolExpr::Infix {
            left: self.expr(),
            op,
            right: ValueExpr::Param(param),
            negated: false,
        }
    }

    fn json_value_infix(self, op: &'static str, param: Param) -> ValueExpr {
        ValueExpr::Binary {
            left: Box::new(self.expr()),
            op: super::ValueOp::Custom(op),
            right: Box::new(ValueExpr::Param(param)),
        }
    }
}
