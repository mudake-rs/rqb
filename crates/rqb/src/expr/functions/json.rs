use crate::{BoolExpr, ValueExpr, ValueOp};

use super::function;

/// Builds `to_json(expr)`.
pub fn to_json(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("to_json", [expr])
}

/// Builds `to_jsonb(expr)`.
pub fn to_jsonb(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("to_jsonb", [expr])
}

/// Builds `jsonb_build_object(...)`.
pub fn jsonb_build_object(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("jsonb_build_object", args)
}

/// Builds `json_build_object(...)`.
pub fn json_build_object(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("json_build_object", args)
}

/// Builds `jsonb_build_array(...)`.
pub fn jsonb_build_array(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("jsonb_build_array", args)
}

/// Builds `json_build_array(...)`.
pub fn json_build_array(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("json_build_array", args)
}

/// Builds `jsonb_object(expr)`.
pub fn jsonb_object(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_object", [expr])
}

/// Builds `json_object(expr)`.
pub fn json_object(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_object", [expr])
}

/// Builds `jsonb_pretty(expr)`.
pub fn jsonb_pretty(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_pretty", [expr])
}

/// Builds `row_to_json(row)`.
pub fn row_to_json(row: impl Into<ValueExpr>) -> ValueExpr {
    function("row_to_json", [row])
}

/// Builds `array_to_json(array)`.
pub fn array_to_json(array: impl Into<ValueExpr>) -> ValueExpr {
    function("array_to_json", [array])
}

/// Builds `jsonb_set(target, path, new_value)`.
pub fn jsonb_set(
    target: impl Into<ValueExpr>,
    path: impl Into<ValueExpr>,
    new_value: impl Into<ValueExpr>,
) -> ValueExpr {
    function("jsonb_set", [target.into(), path.into(), new_value.into()])
}

/// Builds `jsonb_insert(target, path, new_value)`.
pub fn jsonb_insert(
    target: impl Into<ValueExpr>,
    path: impl Into<ValueExpr>,
    new_value: impl Into<ValueExpr>,
) -> ValueExpr {
    function(
        "jsonb_insert",
        [target.into(), path.into(), new_value.into()],
    )
}

/// Builds the Postgres `jsonb - key` delete expression.
pub fn jsonb_delete(target: impl Into<ValueExpr>, key: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("-"),
        right: Box::new(key.into()),
    }
}

/// Builds `jsonb_strip_nulls(expr)`.
pub fn jsonb_strip_nulls(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_strip_nulls", [expr])
}

/// Builds `jsonb_typeof(expr)`.
pub fn jsonb_typeof(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_typeof", [expr])
}

/// Builds `json_typeof(expr)`.
pub fn json_typeof(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_typeof", [expr])
}

/// Builds `json_array_length(expr)`.
pub fn json_array_length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_array_length", [expr])
}

/// Builds `jsonb_array_length(expr)`.
pub fn jsonb_array_length(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_array_length", [expr])
}

/// Builds `jsonb_array_elements(expr)`.
pub fn jsonb_array_elements(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_array_elements", [expr])
}

/// Builds `jsonb_each(expr)`.
pub fn jsonb_each(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_each", [expr])
}

/// Builds `jsonb_path_query(target, path)`.
pub fn jsonb_path_query(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_path_query", [target.into(), path.into()])
}

/// Builds `jsonb_path_exists(target, path)` as a boolean predicate.
pub fn jsonb_path_exists(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> BoolExpr {
    function("jsonb_path_exists", [target.into(), path.into()]).is_true()
}

/// Builds the Postgres `target -> key` JSON access expression.
pub fn json_get(target: impl Into<ValueExpr>, key: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("->"),
        right: Box::new(key.into()),
    }
}

/// Builds the Postgres `target ->> key` JSON text access expression.
pub fn json_get_text(target: impl Into<ValueExpr>, key: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("->>"),
        right: Box::new(key.into()),
    }
}

/// Builds the Postgres `target #> path` JSON path expression.
pub fn json_path(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("#>"),
        right: Box::new(path.into()),
    }
}

/// Builds the Postgres `target #>> path` JSON path text expression.
pub fn json_path_text(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("#>>"),
        right: Box::new(path.into()),
    }
}

/// Builds the SQL/JSON `json(expr)` constructor.
pub fn json(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json", [expr])
}

/// Builds the SQL/JSON `json_scalar(expr)` constructor.
pub fn json_scalar(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_scalar", [expr])
}

/// Builds `json_serialize(expr)`.
pub fn json_serialize(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_serialize", [expr])
}

/// Builds `json_exists(target, path)` as a boolean predicate.
pub fn json_exists(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> BoolExpr {
    function("json_exists", [target.into(), path.into()]).is_true()
}

/// Builds `json_query(target, path)`.
pub fn json_query(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    function("json_query", [target.into(), path.into()])
}

/// Builds `json_value(target, path)`.
pub fn json_value(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    function("json_value", [target.into(), path.into()])
}
