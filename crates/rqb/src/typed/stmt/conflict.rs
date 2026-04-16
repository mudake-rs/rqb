use super::*;

impl ColumnConflictBuilder {
    pub fn and<T>(mut self, field: Field<T>) -> Self {
        push_column(&mut self.fields, *field.meta);
        self
    }

    /// Adds an index predicate to an `ON CONFLICT (columns...)` target.
    ///
    /// Repeated calls are AND-combined, matching repeated `filter(...)` calls.
    pub fn target_where(mut self, predicate: BoolExpr) -> Self {
        self.predicate = Some(Box::new(BoolExpr::and_option(
            self.predicate.take().map(|existing| *existing),
            predicate,
        )));
        self
    }

    pub fn do_nothing(self) -> Insert {
        finish_conflict(
            self.insert,
            column_target(self.fields, self.predicate),
            ConflictAction::DoNothing,
        )
    }

    pub fn do_update_set<I>(self, assignments: I) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        finish_conflict(
            self.insert,
            column_target(self.fields, self.predicate),
            update_action(assignments, None),
        )
    }

    pub fn do_update_set_where<I>(self, assignments: I, filter: BoolExpr) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        finish_conflict(
            self.insert,
            column_target(self.fields, self.predicate),
            update_action(assignments, Some(filter)),
        )
    }
}

impl ConstraintConflictBuilder {
    pub fn do_nothing(self) -> Insert {
        finish_conflict(
            self.insert,
            ConflictTarget::Constraint(self.constraint),
            ConflictAction::DoNothing,
        )
    }

    pub fn do_update_set<I>(self, assignments: I) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        finish_conflict(
            self.insert,
            ConflictTarget::Constraint(self.constraint),
            update_action(assignments, None),
        )
    }

    pub fn do_update_set_where<I>(self, assignments: I, filter: BoolExpr) -> Insert
    where
        I: IntoIterator<Item = Assignment>,
    {
        finish_conflict(
            self.insert,
            ConflictTarget::Constraint(self.constraint),
            update_action(assignments, Some(filter)),
        )
    }
}

fn column_target(fields: Vec<Meta>, predicate: Option<Box<BoolExpr>>) -> ConflictTarget {
    ConflictTarget::Columns { fields, predicate }
}

fn update_action(
    assignments: impl IntoIterator<Item = Assignment>,
    filter: Option<BoolExpr>,
) -> ConflictAction {
    ConflictAction::DoUpdate {
        assignments: assignments.into_iter().collect(),
        filter: filter.map(Box::new),
    }
}

fn finish_conflict(mut insert: Insert, target: ConflictTarget, action: ConflictAction) -> Insert {
    insert.conflict = Some(ConflictClause { target, action });
    insert
}
