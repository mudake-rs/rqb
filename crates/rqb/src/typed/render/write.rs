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

    pub(super) fn render_update(&mut self, update: &crate::typed::Update) -> Result<()> {
        self.sql.push_str("UPDATE ");
        self.render_write_target(&update.target);
        self.sql.push_str(" SET ");
        self.render_assignments(&update.assignments)?;
        if let Some(filter) = &update.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_returning(&update.returning)?;
        Ok(())
    }

    pub(super) fn render_delete(&mut self, delete: &Delete) -> Result<()> {
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&delete.target);
        if let Some(filter) = &delete.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter)?;
        }
        self.render_returning(&delete.returning)?;
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
            ConflictTarget::Invalid { .. } => unreachable!("invalid conflict target validated"),
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
