use crate::typed::ValueExpr;

use super::function;

pub fn merge_action() -> ValueExpr {
    function("merge_action", Vec::<ValueExpr>::new())
}
