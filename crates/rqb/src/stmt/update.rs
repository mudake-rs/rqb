use super::*;

impl Update {
    /// Creates an update statement for a table or view source.
    pub(crate) fn table(target: impl Into<Source>) -> Self {
        Self {
            ctes: Vec::new(),
            target: target.into(),
            assignments: Vec::new(),
            from: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    /// Adds a CTE to the update statement.
    #[inline]
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Adds or replaces one `SET` assignment.
    ///
    /// Later assignments for the same database column replace earlier ones, so
    /// service code can layer server-owned values after a request changeset.
    #[inline]
    pub fn set(mut self, assignment: Assignment) -> Self {
        push_assignment(&mut self.assignments, assignment);
        self
    }

    /// Adds or replaces multiple `SET` assignments.
    ///
    /// Later assignments for the same database column replace earlier ones.
    pub fn set_many(mut self, assignments: impl IntoAssignments) -> Self {
        extend_assignments(&mut self.assignments, assignments.into_assignments());
        self
    }

    /// Adds one `SET` assignment only when `condition` is true.
    #[inline]
    pub fn set_if(self, condition: bool, assignment: Assignment) -> Self {
        if condition {
            self.set(assignment)
        } else {
            self
        }
    }

    /// Adds one `SET` assignment built from an optional value.
    pub fn set_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> Assignment) -> Self {
        match value {
            Some(value) => self.set(f(value)),
            None => self,
        }
    }

    /// Applies assignments produced by a partial update [`Changeset`] DTO.
    ///
    /// Assignments are merged with replacement semantics. Call `patch(&dto)`
    /// first, then `set(...)` for authenticated/server-owned values that must
    /// override request data.
    pub fn patch(mut self, changes: impl Changeset) -> Self {
        extend_assignments(&mut self.assignments, changes.changeset_assignments());
        self
    }

    /// Adds a `FROM` source.
    pub fn from(mut self, source: impl Into<Source>) -> Self {
        self.from.push(source.into());
        self
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

impl_filter_methods!(Update);
