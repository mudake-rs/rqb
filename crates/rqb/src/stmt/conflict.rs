use super::*;

impl<I> ColumnConflictBuilder<I> {
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
}

macro_rules! impl_conflict_actions {
    ($host:ty $(, $field:tt)?) => {
        impl ColumnConflictBuilder<$host> {
            /// Finishes the conflict clause with `DO NOTHING`.
            #[inline]
            pub fn do_nothing(self) -> $host {
                self.finish(ConflictAction::DoNothing)
            }

            /// Finishes the conflict clause with `DO UPDATE SET`.
            pub fn do_update_set(self, assignments: impl IntoAssignments) -> $host {
                self.finish(update_action(assignments, None))
            }

            /// Finishes the conflict clause with `DO UPDATE SET field = EXCLUDED.field`.
            pub fn do_update_excluded(self, fields: impl IntoFieldMetas) -> $host {
                self.do_update_set(excluded_assignments(fields))
            }

            /// Finishes the conflict clause with `DO UPDATE SET ... WHERE`.
            pub fn do_update_set_where(
                self,
                assignments: impl IntoAssignments,
                filter: BoolExpr,
            ) -> $host {
                self.finish(update_action(assignments, Some(filter)))
            }

            /// Finishes the conflict clause with `DO UPDATE SET field = EXCLUDED.field WHERE ...`.
            pub fn do_update_excluded_where(self, fields: impl IntoFieldMetas, filter: BoolExpr) -> $host {
                self.do_update_set_where(excluded_assignments(fields), filter)
            }
            fn finish(mut self, action: ConflictAction) -> $host {
                self.insert $(.$field)?.conflict = Some(ConflictClause {
                    target: ConflictTarget::Columns {
                        fields: self.fields,
                        predicate: self.predicate,
                    },
                    action,
                });
                self.insert
            }
        }

        impl ConstraintConflictBuilder<$host> {
            /// Finishes the constraint conflict clause with `DO NOTHING`.
            #[inline]
            pub fn do_nothing(self) -> $host {
                self.finish(ConflictAction::DoNothing)
            }

            /// Finishes the constraint conflict clause with `DO UPDATE SET`.
            pub fn do_update_set(self, assignments: impl IntoAssignments) -> $host {
                self.finish(update_action(assignments, None))
            }

            /// Finishes the constraint conflict clause with `DO UPDATE SET field = EXCLUDED.field`.
            pub fn do_update_excluded(self, fields: impl IntoFieldMetas) -> $host {
                self.do_update_set(excluded_assignments(fields))
            }

            /// Finishes the constraint conflict clause with `DO UPDATE SET ... WHERE`.
            pub fn do_update_set_where(
                self,
                assignments: impl IntoAssignments,
                filter: BoolExpr,
            ) -> $host {
                self.finish(update_action(assignments, Some(filter)))
            }

            /// Finishes the constraint conflict clause with `DO UPDATE SET field = EXCLUDED.field WHERE ...`.
            pub fn do_update_excluded_where(self, fields: impl IntoFieldMetas, filter: BoolExpr) -> $host {
                self.do_update_set_where(excluded_assignments(fields), filter)
            }
            fn finish(mut self, action: ConflictAction) -> $host {
                self.insert $(.$field)?.conflict = Some(ConflictClause {
                    target: ConflictTarget::Constraint(self.constraint),
                    action,
                });
                self.insert
            }
        }

    };
}

impl_conflict_actions!(Insert);
impl_conflict_actions!(InsertRow, 0);

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
