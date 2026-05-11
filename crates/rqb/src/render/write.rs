use super::*;

impl Renderer {
    pub(super) fn render_insert(&mut self, insert: &Insert) -> Result<()> {
        self.sql.push_str("INSERT INTO ");
        self.render_write_target(&insert.target);
        self.sql.push_str(" (");
        if insert.source.is_some() {
            for (idx, field) in insert.columns.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                write_quoted_ident(&mut self.sql, field.db);
            }
        } else {
            for (idx, assignment) in insert.assignments.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                write_quoted_ident(&mut self.sql, assignment.field.db);
            }
        }
        self.sql.push(')');
        if let Some(source) = &insert.source {
            self.sql.push(' ');
            self.render_select(source)?;
        } else {
            self.sql.push_str(" VALUES (");
            for (idx, assignment) in insert.assignments.iter().enumerate() {
                if idx > 0 {
                    self.sql.push_str(", ");
                }
                self.render_value(&assignment.value)?;
            }
            self.sql.push(')');
        }
        if let Some(conflict) = &insert.conflict {
            self.render_conflict(conflict)?;
        }
        self.render_returning(&insert.returning)?;
        Ok(())
    }

    pub(super) fn render_update(&mut self, update: &crate::Update) -> Result<()> {
        self.render_ctes(&update.ctes)?;
        self.sql.push_str("UPDATE ");
        self.render_write_target(&update.target);
        self.sql.push_str(" SET ");
        self.render_assignments(&update.assignments)?;
        self.render_update_from(&update.from)?;
        if let Some(filter) = &update.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_returning(&update.returning)?;
        Ok(())
    }

    pub(super) fn render_delete(&mut self, delete: &Delete) -> Result<()> {
        self.render_ctes(&delete.ctes)?;
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&delete.target);
        self.render_delete_using(&delete.using)?;
        if let Some(filter) = &delete.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_returning(&delete.returning)?;
        Ok(())
    }

    pub(super) fn render_merge(&mut self, merge: &Merge) -> Result<()> {
        self.render_ctes(&merge.ctes)?;
        self.sql.push_str("MERGE INTO ");
        self.render_merge_target(&merge.target);
        self.sql.push_str(" USING ");
        self.render_source(&merge.using)?;
        self.sql.push_str(" ON ");
        self.render_bool(&merge.on)?;
        for action in &merge.actions {
            self.render_merge_action(action)?;
        }
        self.render_returning(&merge.returning)?;
        Ok(())
    }

    pub(super) fn render_merge_target(&mut self, target: &Source) {
        match target {
            Source::Table { name, alias, .. } | Source::View { name, alias, .. } => {
                write_quoted_qualified(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Cte { name, alias, .. } => {
                write_quoted_ident(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Subquery { .. } | Source::Raw { .. } | Source::Function { .. } => {
                unreachable!("merge target validated as table")
            }
        }
    }

    pub(super) fn render_merge_action(&mut self, action: &MergeAction) -> Result<()> {
        match action {
            MergeAction::DoNothing { when, condition } => {
                self.render_merge_when(*when, condition.as_deref())?;
                self.sql.push_str(" THEN DO NOTHING");
                Ok(())
            }
            MergeAction::Insert {
                when,
                condition,
                assignments,
            } => {
                self.render_merge_when(*when, condition.as_deref())?;
                self.sql.push_str(" THEN INSERT (");
                for (idx, assignment) in assignments.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    write_quoted_ident(&mut self.sql, assignment.field.db);
                }
                self.sql.push_str(") VALUES (");
                for (idx, assignment) in assignments.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_value(&assignment.value)?;
                }
                self.sql.push(')');
                Ok(())
            }
            MergeAction::Update {
                when,
                condition,
                assignments,
            } => {
                self.render_merge_when(*when, condition.as_deref())?;
                self.sql.push_str(" THEN UPDATE SET ");
                self.render_assignments(assignments)
            }
            MergeAction::Delete { when, condition } => {
                self.render_merge_when(*when, condition.as_deref())?;
                self.sql.push_str(" THEN DELETE");
                Ok(())
            }
        }
    }

    pub(super) fn render_merge_when(
        &mut self,
        when: MergeWhen,
        condition: Option<&BoolExpr>,
    ) -> Result<()> {
        self.sql.push_str(" WHEN ");
        self.sql.push_str(when.as_sql());
        if let Some(condition) = condition {
            self.sql.push_str(" AND ");
            self.render_bool(condition)?;
        }
        Ok(())
    }

    pub(super) fn render_update_from(&mut self, sources: &[Source]) -> Result<()> {
        if sources.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" FROM ");
        self.render_source_list(sources)
    }

    pub(super) fn render_delete_using(&mut self, sources: &[Source]) -> Result<()> {
        if sources.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" USING ");
        self.render_source_list(sources)
    }

    pub(super) fn render_source_list(&mut self, sources: &[Source]) -> Result<()> {
        for (idx, source) in sources.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_source(source)?;
        }
        Ok(())
    }
    pub(super) fn render_assignments(&mut self, assignments: &[Assignment]) -> Result<()> {
        for (idx, assignment) in assignments.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, assignment.field.db);
            self.sql.push_str(" = ");
            self.render_value(&assignment.value)?;
        }
        Ok(())
    }

    pub(super) fn render_returning(&mut self, returning: &[SelectItem]) -> Result<()> {
        if returning.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" RETURNING ");
        for (idx, item) in returning.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_select_item(item)?;
        }
        Ok(())
    }

    pub(super) fn render_conflict(&mut self, conflict: &ConflictClause) -> Result<()> {
        self.sql.push_str(" ON CONFLICT ");
        match &conflict.target {
            ConflictTarget::Columns { fields, predicate } => {
                self.sql.push('(');
                for (idx, field) in fields.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    write_quoted_ident(&mut self.sql, field.db);
                }
                self.sql.push(')');
                if let Some(predicate) = predicate {
                    self.sql.push_str(" WHERE ");
                    self.render_bool(predicate)?;
                }
            }
            ConflictTarget::Constraint(constraint) => {
                self.sql.push_str("ON CONSTRAINT ");
                write_quoted_ident(&mut self.sql, constraint);
            }
        }
        match &conflict.action {
            ConflictAction::DoNothing => {
                self.sql.push_str(" DO NOTHING");
                Ok(())
            }
            ConflictAction::DoUpdate {
                assignments,
                filter,
            } => {
                self.sql.push_str(" DO UPDATE SET ");
                self.render_assignments(assignments)?;
                if let Some(filter) = filter {
                    self.sql.push_str(" WHERE ");
                    self.render_bool(filter)?;
                }
                Ok(())
            }
        }
    }
}
