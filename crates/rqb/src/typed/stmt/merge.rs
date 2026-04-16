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
    pub fn into(target: impl Into<Source>, using: impl Into<Source>, on: BoolExpr) -> Self {
        Self {
            ctes: Vec::new(),
            target: target.into(),
            using: using.into(),
            on,
            actions: Vec::new(),
            returning: Vec::new(),
        }
    }

    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    pub fn when_matched(self) -> MatchedMergeBuilder {
        MatchedMergeBuilder {
            merge: self,
            condition: None,
        }
    }

    pub fn when_matched_if(self, condition: BoolExpr) -> MatchedMergeBuilder {
        MatchedMergeBuilder {
            merge: self,
            condition: Some(Box::new(condition)),
        }
    }

    pub fn when_not_matched(self) -> NotMatchedMergeBuilder {
        NotMatchedMergeBuilder {
            merge: self,
            condition: None,
        }
    }

    pub fn when_not_matched_if(self, condition: BoolExpr) -> NotMatchedMergeBuilder {
        NotMatchedMergeBuilder {
            merge: self,
            condition: Some(Box::new(condition)),
        }
    }

    pub fn when_not_matched_by_source(self) -> NotMatchedBySourceMergeBuilder {
        NotMatchedBySourceMergeBuilder {
            merge: self,
            condition: None,
        }
    }

    pub fn when_not_matched_by_source_if(
        self,
        condition: BoolExpr,
    ) -> NotMatchedBySourceMergeBuilder {
        NotMatchedBySourceMergeBuilder {
            merge: self,
            condition: Some(Box::new(condition)),
        }
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

impl MatchedMergeBuilder {
    pub fn update(self, assignments: impl Into<Vec<Assignment>>) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Update {
                when: MergeWhen::Matched,
                condition: self.condition,
                assignments: assignments.into(),
            },
        )
    }

    pub fn delete(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Delete {
                when: MergeWhen::Matched,
                condition: self.condition,
            },
        )
    }

    pub fn do_nothing(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::DoNothing {
                when: MergeWhen::Matched,
                condition: self.condition,
            },
        )
    }
}

impl NotMatchedMergeBuilder {
    pub fn insert(self, assignments: impl Into<Vec<Assignment>>) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Insert {
                condition: self.condition,
                assignments: assignments.into(),
            },
        )
    }

    pub fn do_nothing(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::DoNothing {
                when: MergeWhen::NotMatched,
                condition: self.condition,
            },
        )
    }
}

impl NotMatchedBySourceMergeBuilder {
    pub fn update(self, assignments: impl Into<Vec<Assignment>>) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Update {
                when: MergeWhen::NotMatchedBySource,
                condition: self.condition,
                assignments: assignments.into(),
            },
        )
    }

    pub fn delete(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Delete {
                when: MergeWhen::NotMatchedBySource,
                condition: self.condition,
            },
        )
    }

    pub fn do_nothing(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::DoNothing {
                when: MergeWhen::NotMatchedBySource,
                condition: self.condition,
            },
        )
    }
}

fn finish_merge_action(mut merge: Merge, action: MergeAction) -> Merge {
    merge.actions.push(action);
    merge
}

impl From<Merge> for Stmt {
    fn from(stmt: Merge) -> Self {
        Self::Merge(Box::new(stmt))
    }
}
