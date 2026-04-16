use super::*;

impl Update {
    pub fn table(target: impl Into<Source>) -> Self {
        Self {
            target: target.into(),
            assignments: Vec::new(),
            from: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn set(mut self, assignment: Assignment) -> Self {
        push_assignment(&mut self.assignments, assignment);
        self
    }

    pub fn changes(mut self, changes: impl Changeset) -> Self {
        extend_assignments(&mut self.assignments, changes.changeset_assignments());
        self
    }

    pub fn from(mut self, source: impl Into<Source>) -> Self {
        self.from.push(source.into());
        self
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(BoolExpr::and_option(self.filter, filter));
        self
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
