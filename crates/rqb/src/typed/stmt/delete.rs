use super::*;

impl Delete {
    /// Creates a delete statement for a table or view source.
    #[doc(hidden)]
    pub fn from(target: impl Into<Source>) -> Self {
        Self {
            target: target.into(),
            using: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    /// Adds a `USING` source.
    pub fn using(mut self, source: impl Into<Source>) -> Self {
        self.using.push(source.into());
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
