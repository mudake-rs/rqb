use crate::expr::Expr;
use crate::field::FieldRef;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictTarget {
    Columns(Vec<FieldRef>),
    Constraint(String),
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ConflictAction {
    DoNothing,
    DoUpdate {
        fields: Vec<FieldRef>,
        filter: Option<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConflictClause {
    pub target: ConflictTarget,
    pub action: ConflictAction,
}
