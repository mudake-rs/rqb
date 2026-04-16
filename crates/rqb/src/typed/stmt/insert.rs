use super::*;

impl Insert {
    pub fn into(target: impl Into<Source>) -> Self {
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
    pub fn set(mut self, assignment: Assignment) -> Self {
        push_column(&mut self.columns, assignment.field);
        push_assignment(&mut self.assignments, assignment);
        self
    }

    pub fn values(mut self, values: impl Insertable) -> Self {
        extend_insert_assignments(
            &mut self.columns,
            &mut self.assignments,
            values.insert_assignments(),
        );
        self
    }

    pub fn column<T>(mut self, field: Field<T>) -> Self {
        push_column(&mut self.columns, *field.meta);
        self
    }

    pub fn from_select(mut self, select: Select) -> Self {
        self.source = Some(Box::new(select));
        self
    }

    pub fn on_conflict(self, fields: impl ConflictFields) -> ColumnConflictBuilder {
        let mut target_fields = Vec::with_capacity(fields.conflict_field_count());
        fields.push_conflict_fields(&mut target_fields);
        ColumnConflictBuilder {
            insert: self,
            fields: target_fields,
            predicate: None,
        }
    }

    pub fn on_conflict_constraint(
        self,
        constraint: impl Into<String>,
    ) -> ConstraintConflictBuilder {
        ConstraintConflictBuilder {
            insert: self,
            constraint: constraint.into(),
        }
    }

    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }
}
