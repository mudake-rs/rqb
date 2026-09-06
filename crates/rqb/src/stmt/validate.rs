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

impl RowLimit {
    fn validate(&self, order: &[OrderItem]) -> Result<()> {
        match self {
            Self::Limit(_) => Ok(()),
            Self::Fetch(fetch) => {
                validate_fetch_shape(order, fetch)?;
                fetch.validate()
            }
        }
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
        for operand in [&self.left, &self.right] {
            operand.validate_query_statement(
                "set query operands must be SELECT, set, or raw statements",
            )?;
            if matches!(operand.as_ref(), Stmt::Select(select) if select.lock.is_some()) {
                return Err(Error::InvalidSelectShape {
                    message: "set query operands cannot have row locks",
                });
            }
        }
        if let Some(count) = self.left.projection_count() {
            self.right.validate_projection_count(
                count,
                "set query operands must return the same column count",
            )?;
        }
        for item in &self.order {
            item.validate()?;
        }
        if let Some(row_limit) = &self.row_limit {
            row_limit.validate(&self.order)?;
        }
        Ok(())
    }
}

impl Select {
    /// Validates source, joins, expressions, limits, locks, and CTEs.
    pub fn validate(&self) -> Result<()> {
        if let Some(lock) = &self.lock {
            if self.distinct
                || !self.distinct_on.is_empty()
                || !self.group_by.is_empty()
                || self.having.is_some()
                || self
                    .projection
                    .iter()
                    .any(|item| item.expr.prevents_row_lock())
                || self.order.iter().any(|item| item.expr.prevents_row_lock())
            {
                return Err(Error::InvalidSelectShape {
                    message: "row locks cannot be combined with DISTINCT, grouping, aggregates or window functions",
                });
            }
            if lock.of.iter().any(|alias| alias.is_empty()) {
                return Err(Error::InvalidSelectShape {
                    message: "row lock relation cannot be empty",
                });
            }
        }
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
            validate_select_item(item, "projection alias cannot be empty")?;
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
        if let Some(row_limit) = &self.row_limit {
            row_limit.validate(&self.order)?;
        }
        Ok(())
    }
}

impl Insert {
    /// Validates target, assignments, conflict handling, and returning list.
    pub fn validate(&self) -> Result<()> {
        validate_cte_names(&self.ctes)?;
        for cte in &self.ctes {
            cte.validate()?;
        }
        validate_table_target("insert", &self.target)?;
        match &self.body {
            InsertBody::Values(assignments) => {
                validate_nonempty_assignments("insert", assignments)?;
                for assignment in assignments {
                    validate_assignment_value(&assignment.value)?;
                }
            }
            InsertBody::Select { columns, select } => {
                validate_nonempty_columns("insert-select", columns)?;
                validate_insert_select_columns(columns, select)?;
                select.validate()?;
            }
            InsertBody::DefaultValues => {}
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
                validate_assignment_value(&assignment.value)?;
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
            validate_assignment_value(&assignment.value)?;
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
    if target.is_table_or_view() {
        return target.validate();
    }
    Err(Error::invalid_write_target(statement, target.kind()))
}

fn validate_fetch_shape(order: &[OrderItem], fetch: &FetchClause) -> Result<()> {
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

fn validate_assignment_value(value: &AssignmentValue) -> Result<()> {
    match value {
        AssignmentValue::Expr(expr) => expr.validate(),
        AssignmentValue::Default => Ok(()),
    }
}

fn validate_nonempty_columns(statement: &'static str, columns: &[Meta]) -> Result<()> {
    if columns.is_empty() {
        return Err(Error::EmptyColumns { statement });
    }
    Ok(())
}

impl MergeAction {
    fn validate(&self) -> Result<()> {
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
                    validate_assignment_value(&assignment.value)?;
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
                    validate_assignment_value(&assignment.value)?;
                }
                Ok(())
            }
        }
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
        let mut exhausted = [false; 3];
        for action in &self.actions {
            let (when, condition) = match action {
                MergeAction::DoNothing { when, condition }
                | MergeAction::Insert {
                    when, condition, ..
                }
                | MergeAction::Update {
                    when, condition, ..
                }
                | MergeAction::Delete { when, condition } => (when, condition),
            };
            let index = match when {
                MergeWhen::Matched => 0,
                MergeWhen::NotMatched => 1,
                MergeWhen::NotMatchedBySource => 2,
            };
            if exhausted[index] {
                return Err(Error::InvalidMergeShape {
                    message: "unreachable WHEN after unconditional branch of the same kind",
                });
            }
            exhausted[index] = condition.is_none();
            action.validate()?;
        }
        validate_returning(&self.returning)
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
            return (!self
                .projection
                .iter()
                .any(|item| matches!(item.expr, ValueExpr::Raw { .. })))
            .then_some(self.projection.len());
        }
        let mut count = 0usize;
        self.source.for_each_field(|_| count += 1);
        (count > 0).then_some(count)
    }
}

impl Stmt {
    pub(crate) fn validate_projection_count(
        &self,
        expected: usize,
        message: &'static str,
    ) -> Result<()> {
        if self
            .projection_count()
            .is_some_and(|count| count != expected)
        {
            return Err(Error::InvalidSelectShape { message });
        }
        Ok(())
    }

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
            Self::Set(set) => set
                .left
                .projection_count()
                .or_else(|| set.right.projection_count()),
            Self::Raw(_) => None,
        }
    }
}

fn validate_returning(returning: &[SelectItem]) -> Result<()> {
    for item in returning {
        validate_select_item(item, "returning alias cannot be empty")?;
    }
    Ok(())
}

fn validate_select_item(item: &SelectItem, alias_message: &'static str) -> Result<()> {
    item.expr.validate()?;
    if matches!(item.alias.as_deref(), Some("")) {
        return Err(Error::InvalidSelectShape {
            message: alias_message,
        });
    }
    Ok(())
}
