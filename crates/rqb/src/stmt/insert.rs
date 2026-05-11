use super::*;

impl Insert {
    /// Creates an insert statement for a table or view source.
    pub(crate) fn into(target: impl Into<Source>) -> Self {
        Self {
            target: target.into(),
            columns: Vec::new(),
            assignments: Vec::new(),
            source: None,
            conflict: None,
            returning: Vec::new(),
        }
    }

    /// Adds one column assignment. If the same database column was assigned
    /// earlier, this assignment replaces the earlier value.
    ///
    /// This makes it safe to layer server-owned values around a DTO mapping:
    /// call `values(&dto)` for request-owned fields and use `set(...)` for
    /// generated IDs, tenant IDs, status defaults, or explicit overrides.
    #[inline]
    pub fn set(mut self, assignment: Assignment) -> Self {
        push_column(&mut self.columns, assignment.field);
        push_assignment(&mut self.assignments, assignment);
        self
    }

    /// Adds multiple column assignments. Later assignments for the same
    /// database column replace earlier values.
    pub fn set_many(mut self, assignments: impl IntoAssignments) -> Self {
        extend_insert_assignments(
            &mut self.columns,
            &mut self.assignments,
            assignments.into_assignments(),
        );
        self
    }

    /// Adds one assignment only when `condition` is true.
    #[inline]
    pub fn set_if(self, condition: bool, assignment: Assignment) -> Self {
        if condition {
            self.set(assignment)
        } else {
            self
        }
    }

    /// Adds one assignment built from an optional value.
    pub fn set_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> Assignment) -> Self {
        match value {
            Some(value) => self.set(f(value)),
            None => self,
        }
    }

    /// Adds assignments produced by an [`Insertable`] DTO.
    pub fn values(mut self, values: impl Insertable) -> Self {
        extend_insert_assignments(
            &mut self.columns,
            &mut self.assignments,
            values.insert_assignments(),
        );
        self
    }

    /// Adds a target column for `INSERT ... SELECT`.
    pub fn column<T>(mut self, field: Field<T>) -> Self {
        push_column(&mut self.columns, *field.meta);
        self
    }

    /// Adds multiple target columns for `INSERT ... SELECT`.
    pub fn columns(mut self, fields: impl IntoFieldMetas) -> Self {
        for field in fields.into_field_metas() {
            push_column(&mut self.columns, field);
        }
        self
    }

    /// Uses a select statement as the insert source.
    #[inline]
    pub fn from_select(mut self, select: Select) -> Self {
        self.source = Some(Box::new(select));
        self
    }

    /// Starts an `ON CONFLICT (columns...)` clause.
    pub fn on_conflict(self, fields: impl ConflictFields) -> ColumnConflictBuilder {
        let mut target_fields = Vec::with_capacity(fields.conflict_field_count());
        fields.push_conflict_fields(&mut target_fields);
        ColumnConflictBuilder {
            insert: self,
            fields: target_fields,
            predicate: None,
        }
    }

    /// Starts an `ON CONFLICT ON CONSTRAINT` clause.
    pub fn on_conflict_constraint(
        self,
        constraint: impl Into<String>,
    ) -> ConstraintConflictBuilder {
        ConstraintConflictBuilder {
            insert: self,
            constraint: constraint.into(),
        }
    }

    /// Adds one field to `RETURNING`.
    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    /// Replaces `RETURNING` with every field exposed by the target source.
    #[inline]
    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    /// Adds an arbitrary item to `RETURNING`.
    #[inline]
    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }
}
