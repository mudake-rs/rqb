use super::*;

impl Delete {
    pub fn from(target: Source) -> Self {
        Self {
            target,
            using: Vec::new(),
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn using(mut self, source: Source) -> Self {
        self.using.push(source);
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
