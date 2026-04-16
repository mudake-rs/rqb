use super::*;

impl MergeWhen {
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Matched => "MATCHED",
            Self::NotMatched => "NOT MATCHED",
            Self::NotMatchedBySource => "NOT MATCHED BY SOURCE",
        }
    }
}

impl Merge {
    pub fn into(target: Source, using: Source, on: BoolExpr) -> Self {
        Self {
            ctes: Vec::new(),
            target,
            using,
            on,
            actions: Vec::new(),
            returning: Vec::new(),
        }
    }

    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    pub fn when_matched_update(mut self, assignments: impl Into<Vec<Assignment>>) -> Self {
        self.actions.push(MergeAction::Update {
            when: MergeWhen::Matched,
            condition: None,
            assignments: assignments.into(),
        });
        self
    }

    pub fn when_matched_update_if(
        mut self,
        condition: BoolExpr,
        assignments: impl Into<Vec<Assignment>>,
    ) -> Self {
        self.actions.push(MergeAction::Update {
            when: MergeWhen::Matched,
            condition: Some(Box::new(condition)),
            assignments: assignments.into(),
        });
        self
    }

    pub fn when_matched_delete(mut self) -> Self {
        self.actions.push(MergeAction::Delete {
            when: MergeWhen::Matched,
            condition: None,
        });
        self
    }

    pub fn when_matched_delete_if(mut self, condition: BoolExpr) -> Self {
        self.actions.push(MergeAction::Delete {
            when: MergeWhen::Matched,
            condition: Some(Box::new(condition)),
        });
        self
    }

    pub fn when_matched_do_nothing(mut self) -> Self {
        self.actions.push(MergeAction::DoNothing {
            when: MergeWhen::Matched,
            condition: None,
        });
        self
    }

    pub fn when_matched_do_nothing_if(mut self, condition: BoolExpr) -> Self {
        self.actions.push(MergeAction::DoNothing {
            when: MergeWhen::Matched,
            condition: Some(Box::new(condition)),
        });
        self
    }

    pub fn when_not_matched_insert(mut self, assignments: impl Into<Vec<Assignment>>) -> Self {
        self.actions.push(MergeAction::Insert {
            condition: None,
            assignments: assignments.into(),
        });
        self
    }

    pub fn when_not_matched_insert_if(
        mut self,
        condition: BoolExpr,
        assignments: impl Into<Vec<Assignment>>,
    ) -> Self {
        self.actions.push(MergeAction::Insert {
            condition: Some(Box::new(condition)),
            assignments: assignments.into(),
        });
        self
    }

    pub fn when_not_matched_do_nothing(mut self) -> Self {
        self.actions.push(MergeAction::DoNothing {
            when: MergeWhen::NotMatched,
            condition: None,
        });
        self
    }

    pub fn when_not_matched_do_nothing_if(mut self, condition: BoolExpr) -> Self {
        self.actions.push(MergeAction::DoNothing {
            when: MergeWhen::NotMatched,
            condition: Some(Box::new(condition)),
        });
        self
    }

    pub fn when_not_matched_by_source_update(
        mut self,
        assignments: impl Into<Vec<Assignment>>,
    ) -> Self {
        self.actions.push(MergeAction::Update {
            when: MergeWhen::NotMatchedBySource,
            condition: None,
            assignments: assignments.into(),
        });
        self
    }

    pub fn when_not_matched_by_source_update_if(
        mut self,
        condition: BoolExpr,
        assignments: impl Into<Vec<Assignment>>,
    ) -> Self {
        self.actions.push(MergeAction::Update {
            when: MergeWhen::NotMatchedBySource,
            condition: Some(Box::new(condition)),
            assignments: assignments.into(),
        });
        self
    }

    pub fn when_not_matched_by_source_delete(mut self) -> Self {
        self.actions.push(MergeAction::Delete {
            when: MergeWhen::NotMatchedBySource,
            condition: None,
        });
        self
    }

    pub fn when_not_matched_by_source_delete_if(mut self, condition: BoolExpr) -> Self {
        self.actions.push(MergeAction::Delete {
            when: MergeWhen::NotMatchedBySource,
            condition: Some(Box::new(condition)),
        });
        self
    }

    pub fn when_not_matched_by_source_do_nothing(mut self) -> Self {
        self.actions.push(MergeAction::DoNothing {
            when: MergeWhen::NotMatchedBySource,
            condition: None,
        });
        self
    }

    pub fn when_not_matched_by_source_do_nothing_if(mut self, condition: BoolExpr) -> Self {
        self.actions.push(MergeAction::DoNothing {
            when: MergeWhen::NotMatchedBySource,
            condition: Some(Box::new(condition)),
        });
        self
    }

    pub fn returning(mut self, field: impl Into<SelectItem>) -> Self {
        self.returning.push(field.into());
        self
    }

    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    pub fn returning_all(mut self) -> Self {
        let qualifier = self.target.explicit_alias().map(str::to_owned);
        let mut returning = Vec::new();
        self.target.for_each_field(|field| {
            returning.push(SelectItem {
                expr: ValueExpr::Field {
                    meta: *field,
                    qualifier: qualifier.clone(),
                },
                alias: None,
            });
        });
        self.returning = returning;
        self
    }
}

impl From<Merge> for Stmt {
    fn from(stmt: Merge) -> Self {
        Self::Merge(Box::new(stmt))
    }
}
