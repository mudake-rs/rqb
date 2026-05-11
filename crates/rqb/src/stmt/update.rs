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
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Adds or replaces one `SET` assignment.
    pub fn set(mut self, assignment: Assignment) -> Self {
        push_assignment(&mut self.assignments, assignment);
        self
    }

    /// Adds or replaces multiple `SET` assignments.
    pub fn set_many(mut self, assignments: impl IntoAssignments) -> Self {
        extend_assignments(&mut self.assignments, assignments.into_assignments());
        self
    }

    /// Adds one `SET` assignment only when `condition` is true.
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
    pub fn patch(mut self, changes: impl Changeset) -> Self {
        extend_assignments(&mut self.assignments, changes.changeset_assignments());
        self
    }

    /// Adds a `FROM` source.
    pub fn from(mut self, source: impl Into<Source>) -> Self {
        self.from.push(source.into());
        self
    }

    /// Adds a `WHERE` predicate, composing with existing predicates using `AND`.
    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(BoolExpr::and_option(self.filter, filter));
        self
    }

    /// Adds a `WHERE` predicate only when `condition` is true.
    pub fn filter_if(self, condition: bool, filter: BoolExpr) -> Self {
        if condition { self.filter(filter) } else { self }
    }

    /// Adds a `WHERE` predicate built from an optional value.
    pub fn filter_option<T>(self, value: Option<T>, f: impl FnOnce(T) -> BoolExpr) -> Self {
        match value {
            Some(value) => self.filter(f(value)),
            None => self,
        }
    }

    /// Adds one field to `RETURNING`.
    pub fn returning<T>(mut self, field: Field<T>) -> Self {
        self.returning.push(select_item_for_field(field));
        self
    }

    /// Replaces `RETURNING` with every field exposed by the target source.
    pub fn returning_all(mut self) -> Self {
        self.returning.clear();
        push_all_source_fields(&self.target, &mut self.returning);
        self
    }

    /// Adds an arbitrary item to `RETURNING`.
    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }
}
