use crate::typed::{BoolExpr, ValueExpr, ValueOp};

use super::function;

pub fn to_json(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("to_json", [expr])
}

pub fn to_jsonb(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("to_jsonb", [expr])
}

pub fn jsonb_build_object(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("jsonb_build_object", args)
}

pub fn jsonb_build_array(args: impl IntoIterator<Item = impl Into<ValueExpr>>) -> ValueExpr {
    function("jsonb_build_array", args)
}

pub fn jsonb_object(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_object", [expr])
}

pub fn jsonb_set(
    target: impl Into<ValueExpr>,
    path: impl Into<ValueExpr>,
    new_value: impl Into<ValueExpr>,
) -> ValueExpr {
    function("jsonb_set", [target.into(), path.into(), new_value.into()])
}

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

pub fn jsonb_delete(target: impl Into<ValueExpr>, key: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("-"),
        right: Box::new(key.into()),
    }
}

pub fn jsonb_strip_nulls(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_strip_nulls", [expr])
}

pub fn jsonb_typeof(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_typeof", [expr])
}

pub fn jsonb_array_elements(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_array_elements", [expr])
}

pub fn jsonb_each(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_each", [expr])
}

pub fn jsonb_path_query(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    function("jsonb_path_query", [target.into(), path.into()])
}

pub fn jsonb_path_exists(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> BoolExpr {
    function("jsonb_path_exists", [target.into(), path.into()]).is_true()
}

pub fn json_get(target: impl Into<ValueExpr>, key: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("->"),
        right: Box::new(key.into()),
    }
}

pub fn json_get_text(target: impl Into<ValueExpr>, key: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("->>"),
        right: Box::new(key.into()),
    }
}

pub fn json_path(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("#>"),
        right: Box::new(path.into()),
    }
}

pub fn json_path_text(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    ValueExpr::Binary {
        left: Box::new(target.into()),
        op: ValueOp::Custom("#>>"),
        right: Box::new(path.into()),
    }
}

pub fn json(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json", [expr])
}

pub fn json_scalar(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_scalar", [expr])
}

pub fn json_serialize(expr: impl Into<ValueExpr>) -> ValueExpr {
    function("json_serialize", [expr])
}

pub fn json_exists(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> BoolExpr {
    function("json_exists", [target.into(), path.into()]).is_true()
}

pub fn json_query(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    function("json_query", [target.into(), path.into()])
}

pub fn json_value(target: impl Into<ValueExpr>, path: impl Into<ValueExpr>) -> ValueExpr {
    function("json_value", [target.into(), path.into()])
}
