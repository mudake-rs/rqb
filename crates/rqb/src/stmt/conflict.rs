use super::*;

impl ColumnConflictBuilder {
    /// Adds another column to the conflict target.
    pub fn column<T>(mut self, field: Field<T>) -> Self {
        push_column(&mut self.fields, *field.meta);
        self
    }

    /// Adds an index predicate to an `ON CONFLICT (columns...)` target.
    ///
    /// This is for partial unique indexes. Repeated calls are AND-combined,
    /// matching repeated `filter(...)` calls. Constraint targets do not support
    /// target predicates.
    #[inline]
    pub fn target_where(mut self, predicate: BoolExpr) -> Self {
        self.predicate = Some(Box::new(BoolExpr::and_option(
            self.predicate.take().map(|existing| *existing),
            predicate,
        )));
        self
    }

    /// Finishes the conflict clause with `DO NOTHING`.
    #[inline]
    pub fn do_nothing(self) -> Insert {
        finish_conflict(
            self.insert,
            column_target(self.fields, self.predicate),
            ConflictAction::DoNothing,
        )
    }

    /// Finishes the conflict clause with `DO UPDATE SET`.
    pub fn do_update_set(self, assignments: impl IntoAssignments) -> Insert {
        finish_conflict(
            self.insert,
            column_target(self.fields, self.predicate),
            update_action(assignments, None),
        )
    }

    /// Finishes the conflict clause with `DO UPDATE SET field = EXCLUDED.field`.
    pub fn do_update_excluded(self, fields: impl IntoFieldMetas) -> Insert {
        self.do_update_set(excluded_assignments(fields))
    }

    /// Finishes the conflict clause with `DO UPDATE SET ... WHERE`.
    pub fn do_update_set_where(
        self,
        assignments: impl IntoAssignments,
        filter: BoolExpr,
    ) -> Insert {
        finish_conflict(
            self.insert,
            column_target(self.fields, self.predicate),
            update_action(assignments, Some(filter)),
        )
    }

    /// Finishes the conflict clause with `DO UPDATE SET field = EXCLUDED.field WHERE ...`.
    pub fn do_update_excluded_where(self, fields: impl IntoFieldMetas, filter: BoolExpr) -> Insert {
        self.do_update_set_where(excluded_assignments(fields), filter)
    }
}

impl ConstraintConflictBuilder {
    /// Finishes the constraint conflict clause with `DO NOTHING`.
    #[inline]
    pub fn do_nothing(self) -> Insert {
        finish_conflict(
            self.insert,
            ConflictTarget::Constraint(self.constraint),
            ConflictAction::DoNothing,
        )
    }

    /// Finishes the constraint conflict clause with `DO UPDATE SET`.
    pub fn do_update_set(self, assignments: impl IntoAssignments) -> Insert {
        finish_conflict(
            self.insert,
            ConflictTarget::Constraint(self.constraint),
            update_action(assignments, None),
        )
    }

    /// Finishes the constraint conflict clause with `DO UPDATE SET field = EXCLUDED.field`.
    pub fn do_update_excluded(self, fields: impl IntoFieldMetas) -> Insert {
        self.do_update_set(excluded_assignments(fields))
    }

    /// Finishes the constraint conflict clause with `DO UPDATE SET ... WHERE`.
    pub fn do_update_set_where(
        self,
        assignments: impl IntoAssignments,
        filter: BoolExpr,
    ) -> Insert {
        finish_conflict(
            self.insert,
            ConflictTarget::Constraint(self.constraint),
            update_action(assignments, Some(filter)),
        )
    }

    /// Finishes the constraint conflict clause with `DO UPDATE SET field = EXCLUDED.field WHERE ...`.
    pub fn do_update_excluded_where(self, fields: impl IntoFieldMetas, filter: BoolExpr) -> Insert {
        self.do_update_set_where(excluded_assignments(fields), filter)
    }
}

fn column_target(fields: Vec<Meta>, predicate: Option<Box<BoolExpr>>) -> ConflictTarget {
    ConflictTarget::Columns { fields, predicate }
}

fn update_action(assignments: impl IntoAssignments, filter: Option<BoolExpr>) -> ConflictAction {
    ConflictAction::DoUpdate {
        assignments: normalized_assignments(assignments),
        filter: filter.map(Box::new),
    }
}

fn excluded_assignments(fields: impl IntoFieldMetas) -> Vec<Assignment> {
    fields
        .into_field_metas()
        .into_iter()
        .map(|field| Assignment {
            field,
            value: crate::AssignmentValue::Expr(ValueExpr::Excluded(field)),
        })
        .collect()
}

fn finish_conflict(mut insert: Insert, target: ConflictTarget, action: ConflictAction) -> Insert {
    insert.conflict = Some(ConflictClause { target, action });
    insert
}
