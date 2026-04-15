use super::*;

impl Delete {
    pub fn from(target: Source) -> Self {
        Self {
            target,
            filter: None,
            returning: Vec::new(),
        }
    }

    pub fn filter(mut self, filter: BoolExpr) -> Self {
        self.filter = Some(match self.filter {
            Some(existing) => BoolExpr::And(vec![existing, filter]),
            None => filter,
        });
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
