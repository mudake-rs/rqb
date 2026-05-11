use super::*;

impl OrderItem {
    /// Validates the ordered expression and its field ordering capability.
    pub fn validate(&self) -> Result<()> {
        self.expr.validate()?;
        if let Some(meta) = self.expr.field_meta()
            && !meta.ops.ordering
        {
            return Err(Error::InvalidSort {
                field: meta.api.to_owned(),
            });
        }
        Ok(())
    }
}

impl GroupByItem {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Expr(expr) => expr.validate(),
            Self::Rollup(exprs) | Self::Cube(exprs) => {
                for expr in exprs {
                    expr.validate()?;
                }
                Ok(())
            }
            Self::GroupingSets(sets) => {
                for set in sets {
                    for expr in set {
                        expr.validate()?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl FetchClause {
    fn validate(&self) -> Result<()> {
        self.count.validate()
    }
}

impl Stmt {
    /// Validates this statement before rendering.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Select(select) => select.validate(),
            Self::Set(set) => set.validate(),
            Self::Insert(insert) => insert.validate(),
            Self::Update(update) => update.validate(),
            Self::Delete(delete) => delete.validate(),
            Self::Merge(merge) => merge.validate(),
            Self::Raw(raw_stmt) => raw_stmt.validate(),
        }
    }

    pub(crate) fn validate_query_statement(&self, message: &'static str) -> Result<()> {
        match self {
            Self::Select(_) | Self::Set(_) | Self::Raw(_) => self.validate(),
            Self::Insert(_) | Self::Update(_) | Self::Delete(_) | Self::Merge(_) => {
                Err(Error::InvalidSelectShape { message })
            }
        }
    }
}

impl SetQuery {
    /// Validates both sides and trailing clauses of this set query.
    pub fn validate(&self) -> Result<()> {
        self.left.validate_query_statement(
            "set query operands must be SELECT, set, or raw statements",
        )?;
        self.right.validate_query_statement(
            "set query operands must be SELECT, set, or raw statements",
        )?;
        for item in &self.order {
            item.validate()?;
        }
        if let Some(fetch) = &self.fetch {
            validate_fetch_shape(self.limit.as_ref(), &self.order, fetch)?;
            fetch.validate()?;
        }
        Ok(())
    }
}

impl Select {
    /// Validates source, joins, expressions, limits, locks, and CTEs.
    pub fn validate(&self) -> Result<()> {
        validate_cte_names(&self.ctes)?;
        for cte in &self.ctes {
            cte.validate()?;
        }
        self.source.validate()?;
        for join in &self.joins {
            join.validate()?;
        }
        for expr in &self.distinct_on {
            expr.validate()?;
        }
        for item in &self.projection {
            item.expr.validate()?;
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        for expr in &self.group_by {
            expr.validate()?;
        }
        if let Some(having) = &self.having {
            having.validate()?;
        }
        for item in &self.order {
            item.validate()?;
        }
        if let Some(fetch) = &self.fetch {
            validate_fetch_shape(self.limit.as_ref(), &self.order, fetch)?;
            fetch.validate()?;
        }
        Ok(())
    }
}

impl Insert {
    /// Validates target, assignments, conflict handling, and returning list.
    pub fn validate(&self) -> Result<()> {
        validate_table_target("insert", &self.target)?;
        match (&self.source, self.assignments.is_empty()) {
            (Some(source), true) => {
                validate_nonempty_columns("insert-select", &self.columns)?;
                validate_insert_select_columns(&self.columns, source)?;
                source.validate()?;
            }
            (Some(_), false) => {
                return Err(Error::InvalidInsertShape {
                    message: "insert-select cannot also contain VALUES assignments",
                });
            }
            (None, true) => validate_nonempty_assignments("insert", &self.assignments)?,
            (None, false) => {
                for assignment in &self.assignments {
                    assignment.value.validate()?;
                }
            }
        }
        if let Some(conflict) = &self.conflict {
            conflict.validate()?;
        }
        validate_returning(&self.returning)
    }
}

impl ConflictClause {
    fn validate(&self) -> Result<()> {
        match &self.target {
            ConflictTarget::Columns { fields, predicate } => {
                validate_nonempty_columns("conflict", fields)?;
                if let Some(predicate) = predicate {
                    predicate.validate()?;
                }
            }
            ConflictTarget::Constraint(constraint) if constraint.is_empty() => {
                return Err(Error::InvalidInsertShape {
                    message: "conflict constraint name cannot be empty",
                });
            }
            ConflictTarget::Constraint(_) => {}
        }
        if let ConflictAction::DoUpdate {
            assignments,
            filter,
        } = &self.action
        {
            validate_nonempty_assignments("conflict update", assignments)?;
            for assignment in assignments {
                assignment.value.validate()?;
            }
            if let Some(filter) = filter {
                filter.validate()?;
            }
        }
        Ok(())
    }
}

impl Update {
    /// Validates target, assignments, optional sources, and returning list.
    pub fn validate(&self) -> Result<()> {
        validate_cte_names(&self.ctes)?;
        for cte in &self.ctes {
            cte.validate()?;
        }
        validate_table_target("update", &self.target)?;
        validate_nonempty_assignments("update", &self.assignments)?;
        for assignment in &self.assignments {
            assignment.value.validate()?;
        }
        for source in &self.from {
            source.validate()?;
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        validate_returning(&self.returning)
    }
}

impl Delete {
    /// Validates target, required filter, optional sources, and returning list.
    pub fn validate(&self) -> Result<()> {
        validate_cte_names(&self.ctes)?;
        for cte in &self.ctes {
            cte.validate()?;
        }
        validate_table_target("delete", &self.target)?;
        let Some(filter) = &self.filter else {
            return Err(Error::DeleteWithoutFilter);
        };
        for source in &self.using {
            source.validate()?;
        }
        filter.validate()?;
        validate_returning(&self.returning)
    }
}

impl RawStmt {
    /// Validates raw placeholder count against supplied binds.
    pub fn validate(&self) -> Result<()> {
        raw_sql::validate_bind_count(&self.sql, self.params.len())
    }
}

fn validate_table_target(statement: &'static str, target: &Source) -> Result<()> {
    if matches!(target, Source::Table { .. } | Source::View { .. }) {
        return Ok(());
    }
    Err(Error::invalid_write_target(statement, target.kind()))
}

fn validate_fetch_shape(
    limit: Option<&Param>,
    order: &[OrderItem],
    fetch: &FetchClause,
) -> Result<()> {
    if limit.is_some() {
        return Err(Error::InvalidSelectShape {
            message: "limit and fetch cannot both be set",
        });
    }
    if fetch.with_ties && order.is_empty() {
        return Err(Error::InvalidSelectShape {
            message: "fetch with ties requires order_by",
        });
    }
    Ok(())
}

fn validate_nonempty_assignments(
    statement: &'static str,
    assignments: &[Assignment],
) -> Result<()> {
    if assignments.is_empty() {
        return Err(Error::EmptyAssignments { statement });
    }
    Ok(())
}

fn validate_nonempty_columns(statement: &'static str, columns: &[Meta]) -> Result<()> {
    if columns.is_empty() {
        return Err(Error::EmptyColumns { statement });
    }
    Ok(())
}

impl MergeAction {
    fn validate(&self) -> Result<()> {
        self.validate_when_matrix()?;
        match self {
            Self::DoNothing { condition, .. } | Self::Delete { condition, .. } => {
                if let Some(condition) = condition {
                    condition.validate()?;
                }
                Ok(())
            }
            Self::Insert {
                condition,
                assignments,
                ..
            } => {
                if let Some(condition) = condition {
                    condition.validate()?;
                }
                validate_nonempty_assignments("merge-insert", assignments)?;
                for assignment in assignments {
                    assignment.value.validate()?;
                }
                Ok(())
            }
            Self::Update {
                condition,
                assignments,
                ..
            } => {
                if let Some(condition) = condition {
                    condition.validate()?;
                }
                validate_nonempty_assignments("merge-update", assignments)?;
                for assignment in assignments {
                    assignment.value.validate()?;
                }
                Ok(())
            }
        }
    }

    fn validate_when_matrix(&self) -> Result<()> {
        let invalid_message = match self {
            Self::DoNothing { .. } => None,
            Self::Insert {
                when: MergeWhen::Matched,
                ..
            } => Some("merge insert is not valid for WHEN MATCHED"),
            Self::Insert {
                when: MergeWhen::NotMatchedBySource,
                ..
            } => Some("merge insert is not valid for WHEN NOT MATCHED BY SOURCE"),
            Self::Insert { .. } => None,
            Self::Update {
                when: MergeWhen::NotMatched,
                ..
            } => Some("merge update is not valid for WHEN NOT MATCHED"),
            Self::Delete {
                when: MergeWhen::NotMatched,
                ..
            } => Some("merge delete is not valid for WHEN NOT MATCHED"),
            Self::Update { .. } | Self::Delete { .. } => None,
        };

        if let Some(message) = invalid_message {
            return Err(Error::InvalidMergeShape { message });
        }
        Ok(())
    }
}

impl Merge {
    /// Validates target, source, match predicate, actions, and returning list.
    pub fn validate(&self) -> Result<()> {
        validate_cte_names(&self.ctes)?;
        for cte in &self.ctes {
            cte.validate()?;
        }
        validate_table_target("merge", &self.target)?;
        self.using.validate()?;
        self.on.validate()?;
        if self.actions.is_empty() {
            return Err(Error::InvalidMergeShape {
                message: "merge requires at least one action",
            });
        }
        for action in &self.actions {
            action.validate()?;
        }
        for item in &self.returning {
            item.expr.validate()?;
        }
        Ok(())
    }
}

fn validate_cte_names(ctes: &[Cte]) -> Result<()> {
    let mut seen = Vec::<&str>::new();
    for cte in ctes {
        if seen.contains(&cte.name.as_str()) {
            return Err(Error::invalid_cte_shape(
                cte.name.clone(),
                "duplicate CTE name",
            ));
        }
        seen.push(cte.name.as_str());
    }
    Ok(())
}

fn validate_insert_select_columns(columns: &[Meta], source: &Select) -> Result<()> {
    if let Some(count) = source.projection_count()
        && count != columns.len()
    {
        return Err(Error::InvalidInsertShape {
            message: "insert-select column count must match SELECT projection count",
        });
    }
    Ok(())
}

impl Select {
    pub(crate) fn projection_count(&self) -> Option<usize> {
        if !self.projection.is_empty() {
            return Some(self.projection.len());
        }
        let mut count = 0usize;
        self.source.for_each_field(|_| count += 1);
        (count > 0).then_some(count)
    }
}

impl Stmt {
    pub(crate) fn projection_count(&self) -> Option<usize> {
        match self {
            Self::Select(select) => select.projection_count(),
            Self::Insert(insert) => {
                (!insert.returning.is_empty()).then_some(insert.returning.len())
            }
            Self::Update(update) => {
                (!update.returning.is_empty()).then_some(update.returning.len())
            }
            Self::Delete(delete) => {
                (!delete.returning.is_empty()).then_some(delete.returning.len())
            }
            Self::Merge(merge) => (!merge.returning.is_empty()).then_some(merge.returning.len()),
            Self::Set(_) | Self::Raw(_) => None,
        }
    }
}

fn validate_returning(returning: &[SelectItem]) -> Result<()> {
    for item in returning {
        item.expr.validate()?;
    }
    Ok(())
}
