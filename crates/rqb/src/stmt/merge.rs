use super::*;

impl MergeWhen {
    /// Returns the SQL branch token.
    #[inline]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Matched => "MATCHED",
            Self::NotMatched => "NOT MATCHED",
            Self::NotMatchedBySource => "NOT MATCHED BY SOURCE",
        }
    }
}

impl Merge {
    /// Creates a PostgreSQL `MERGE` statement.
    pub(crate) fn into(target: impl Into<Source>, using: impl Into<Source>, on: BoolExpr) -> Self {
        Self {
            ctes: Vec::new(),
            target: target.into(),
            using: using.into(),
            on,
            actions: Vec::new(),
            returning: Vec::new(),
        }
    }

    /// Adds a CTE to the merge statement.
    #[inline]
    pub fn with(mut self, cte: Cte) -> Self {
        self.ctes.push(cte);
        self
    }

    /// Starts a `WHEN MATCHED` branch.
    #[inline]
    pub fn when_matched(self) -> MatchedMergeBuilder {
        MatchedMergeBuilder {
            merge: self,
            condition: None,
        }
    }

    /// Starts a conditional `WHEN MATCHED AND ...` branch.
    #[inline]
    pub fn when_matched_if(self, condition: BoolExpr) -> MatchedMergeBuilder {
        MatchedMergeBuilder {
            merge: self,
            condition: Some(Box::new(condition)),
        }
    }

    /// Starts a `WHEN NOT MATCHED` branch.
    #[inline]
    pub fn when_not_matched(self) -> NotMatchedMergeBuilder {
        NotMatchedMergeBuilder {
            merge: self,
            condition: None,
        }
    }

    /// Starts a conditional `WHEN NOT MATCHED AND ...` branch.
    #[inline]
    pub fn when_not_matched_if(self, condition: BoolExpr) -> NotMatchedMergeBuilder {
        NotMatchedMergeBuilder {
            merge: self,
            condition: Some(Box::new(condition)),
        }
    }

    /// Starts a `WHEN NOT MATCHED BY SOURCE` branch.
    #[inline]
    pub fn when_not_matched_by_source(self) -> NotMatchedBySourceMergeBuilder {
        NotMatchedBySourceMergeBuilder {
            merge: self,
            condition: None,
        }
    }

    /// Starts a conditional `WHEN NOT MATCHED BY SOURCE AND ...` branch.
    #[inline]
    pub fn when_not_matched_by_source_if(
        self,
        condition: BoolExpr,
    ) -> NotMatchedBySourceMergeBuilder {
        NotMatchedBySourceMergeBuilder {
            merge: self,
            condition: Some(Box::new(condition)),
        }
    }

    /// Adds one item to `RETURNING`.
    pub fn returning(mut self, field: impl Into<SelectItem>) -> Self {
        self.returning.push(field.into());
        self
    }

    /// Adds an arbitrary item to `RETURNING`.
    #[inline]
    pub fn returning_item(mut self, item: SelectItem) -> Self {
        self.returning.push(item);
        self
    }

    /// Replaces `RETURNING` with every field exposed by the target source.
    #[inline]
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
    /// Finishes this branch with `UPDATE SET`.
    pub fn update(self, assignments: impl IntoAssignments) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Update {
                when: MergeWhen::Matched,
                condition: self.condition,
                assignments: assignments.into_assignments(),
            },
        )
    }

    /// Finishes this branch with `DELETE`.
    #[inline]
    pub fn delete(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Delete {
                when: MergeWhen::Matched,
                condition: self.condition,
            },
        )
    }

    /// Finishes this branch with `DO NOTHING`.
    #[inline]
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
    /// Finishes this branch with `INSERT`.
    pub fn insert(self, assignments: impl IntoAssignments) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Insert {
                when: MergeWhen::NotMatched,
                condition: self.condition,
                assignments: assignments.into_assignments(),
            },
        )
    }

    /// Finishes this branch with `DO NOTHING`.
    #[inline]
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
    /// Finishes this branch with `UPDATE SET`.
    pub fn update(self, assignments: impl IntoAssignments) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Update {
                when: MergeWhen::NotMatchedBySource,
                condition: self.condition,
                assignments: assignments.into_assignments(),
            },
        )
    }

    /// Finishes this branch with `DELETE`.
    #[inline]
    pub fn delete(self) -> Merge {
        finish_merge_action(
            self.merge,
            MergeAction::Delete {
                when: MergeWhen::NotMatchedBySource,
                condition: self.condition,
            },
        )
    }

    /// Finishes this branch with `DO NOTHING`.
    #[inline]
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
