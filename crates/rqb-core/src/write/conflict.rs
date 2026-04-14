use crate::expr::Expr;
use crate::field::FieldRef;

#[derive(Clone, Debug, PartialEq)]
pub enum ConflictTarget {
    Columns {
        fields: Vec<FieldRef>,
        predicate: Option<Box<Expr>>,
    },
    Constraint(String),
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ConflictAction {
    DoNothing,
    DoUpdate {
        assignments: Vec<super::WriteAssignment>,
        filter: Option<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConflictClause {
    pub target: ConflictTarget,
    pub action: ConflictAction,
}
