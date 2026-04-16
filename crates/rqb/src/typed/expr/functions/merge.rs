use crate::typed::ValueExpr;

use super::function;

/// Builds the Postgres `merge_action()` helper expression.
pub fn merge_action() -> ValueExpr {
    function("merge_action", Vec::<ValueExpr>::new())
}
