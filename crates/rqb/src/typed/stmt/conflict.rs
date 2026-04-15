use super::*;

impl InsertConflictBuilder {
    /// Adds another column to an `ON CONFLICT (columns...)` target.
    ///
    /// This is valid only after [`Insert::on_conflict`]. If it is called after
    /// [`Insert::on_conflict_constraint`], the insert fails validation before rendering.
    pub fn and<T>(mut self, field: Field<T>) -> Self {
        match &mut self.target {
            ConflictTarget::Columns { fields, .. } => push_column(fields, *field.meta),
            ConflictTarget::Constraint(_) => {
                invalidate_conflict_target(
                    &mut self.target,
                    "and requires on_conflict(column), not on_conflict_constraint",
                );
            }
            ConflictTarget::Invalid { .. } => {}
        }
        self
    }

    /// Adds an index predicate to an `ON CONFLICT (columns...)` target.
    ///
    /// Repeated calls are AND-combined. This is valid only after
    /// [`Insert::on_conflict`]; using it after [`Insert::on_conflict_constraint`]
    /// fails validation before rendering.
    pub fn target_where(mut self, predicate: BoolExpr) -> Self {
        match &mut self.target {
            ConflictTarget::Columns {
                predicate: current, ..
            } => {
                *current = Some(Box::new(match current.take() {
                    Some(existing) => BoolExpr::And(vec![*existing, predicate]),
                    None => predicate,
                }));
            }
            ConflictTarget::Constraint(_) => {
                invalidate_conflict_target(
                    &mut self.target,
                    "target_where requires on_conflict(column), not on_conflict_constraint",
                );
            }
            ConflictTarget::Invalid { .. } => {}
        }
        self
    }

    pub fn do_nothing(mut self) -> Insert {
        self.insert.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoNothing,
        });
        self.insert
    }

    pub fn do_update_set<I>(mut self, assignments: I) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        self.insert.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoUpdate {
                assignments: assignments.into_iter().collect(),
                filter: None,
            },
        });
        self.insert
    }

    pub fn do_update_set_where<I>(mut self, assignments: I, filter: BoolExpr) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        self.insert.conflict = Some(ConflictClause {
            target: self.target,
            action: ConflictAction::DoUpdate {
                assignments: assignments.into_iter().collect(),
                filter: Some(Box::new(filter)),
            },
        });
        self.insert
    }
}

fn invalidate_conflict_target(target: &mut ConflictTarget, message: &'static str) {
    if !matches!(target, ConflictTarget::Invalid { .. }) {
        *target = ConflictTarget::Invalid { message };
    }
}
