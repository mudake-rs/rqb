use super::*;

impl Delete {
    /// Creates a delete statement for a table or view source.
    pub(crate) fn from(target: impl Into<Source>) -> Self {
        Self {
            ctes: Vec::new(),
            target: target.into(),
            using: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    /// Adds a CTE to the delete statement.
    #[inline]
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Adds a `USING` source.
    pub fn using(mut self, source: impl Into<Source>) -> Self {
        self.using.push(source.into());
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

impl_filter_methods!(Delete);
