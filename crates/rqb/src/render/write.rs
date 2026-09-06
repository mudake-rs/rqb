use super::*;

impl Renderer {
    pub(super) fn render_insert(&mut self, insert: &Insert) {
        self.render_ctes(&insert.ctes);
        self.sql.push_str("INSERT INTO ");
        self.render_write_target(&insert.target);
        match &insert.body {
            InsertBody::DefaultValues => self.sql.push_str(" DEFAULT VALUES"),
            InsertBody::Values(assignments) => {
                self.sql.push(' ');
                self.render_parenthesized_idents(
                    assignments.iter().map(|assignment| assignment.field.db),
                );
                self.sql.push_str(" VALUES (");
                for (idx, assignment) in assignments.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_assignment_value(&assignment.value);
                }
                self.sql.push(')');
            }
            InsertBody::Select { columns, select } => {
                self.sql.push(' ');
                self.render_parenthesized_idents(columns.iter().map(|field| field.db));
                self.sql.push(' ');
                self.render_select(select);
            }
        }
        if let Some(conflict) = &insert.conflict {
            self.render_conflict(conflict);
        }
        self.render_returning(&insert.returning);
    }

    pub(super) fn render_update(&mut self, update: &crate::Update) {
        self.render_ctes(&update.ctes);
        self.sql.push_str("UPDATE ");
        self.render_write_target(&update.target);
        self.sql.push_str(" SET ");
        self.render_assignments(&update.assignments);
        self.render_update_from(&update.from);
        if let Some(filter) = &update.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter);
        }
        self.render_returning(&update.returning);
    }

    pub(super) fn render_delete(&mut self, delete: &Delete) {
        self.render_ctes(&delete.ctes);
        self.sql.push_str("DELETE FROM ");
        self.render_write_target(&delete.target);
        self.render_delete_using(&delete.using);
        if let Some(filter) = &delete.filter {
            self.sql.push_str(" WHERE ");
            self.render_bool(filter);
        }
        self.render_returning(&delete.returning);
    }

    pub(super) fn render_merge(&mut self, merge: &Merge) {
        self.render_ctes(&merge.ctes);
        self.sql.push_str("MERGE INTO ");
        self.render_write_target(&merge.target);
        self.sql.push_str(" USING ");
        self.render_source(&merge.using);
        self.sql.push_str(" ON ");
        self.render_bool(&merge.on);
        for action in &merge.actions {
            self.render_merge_action(action);
        }
        self.render_returning(&merge.returning);
    }

    pub(super) fn render_merge_action(&mut self, action: &MergeAction) {
        match action {
            MergeAction::DoNothing { when, condition } => {
                self.render_merge_when(*when, condition.as_deref());
                self.sql.push_str(" THEN DO NOTHING");
            }
            MergeAction::Insert {
                when,
                condition,
                assignments,
            } => {
                self.render_merge_when(*when, condition.as_deref());
                self.sql.push_str(" THEN INSERT ");
                self.render_parenthesized_idents(
                    assignments.iter().map(|assignment| assignment.field.db),
                );
                self.sql.push_str(" VALUES (");
                for (idx, assignment) in assignments.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.render_assignment_value(&assignment.value);
                }
                self.sql.push(')');
            }
            MergeAction::Update {
                when,
                condition,
                assignments,
            } => {
                self.render_merge_when(*when, condition.as_deref());
                self.sql.push_str(" THEN UPDATE SET ");
                self.render_assignments(assignments)
            }
            MergeAction::Delete { when, condition } => {
                self.render_merge_when(*when, condition.as_deref());
                self.sql.push_str(" THEN DELETE");
            }
        }
    }

    pub(super) fn render_merge_when(&mut self, when: MergeWhen, condition: Option<&BoolExpr>) {
        self.sql.push_str(" WHEN ");
        self.sql.push_str(when.as_sql());
        if let Some(condition) = condition {
            self.sql.push_str(" AND ");
            self.render_bool(condition);
        }
    }

    pub(super) fn render_update_from(&mut self, sources: &[Source]) {
        if sources.is_empty() {
            return;
        }
        self.sql.push_str(" FROM ");
        self.render_source_list(sources)
    }

    pub(super) fn render_delete_using(&mut self, sources: &[Source]) {
        if sources.is_empty() {
            return;
        }
        self.sql.push_str(" USING ");
        self.render_source_list(sources)
    }

    pub(super) fn render_source_list(&mut self, sources: &[Source]) {
        for (idx, source) in sources.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_source(source);
        }
    }
    pub(super) fn render_assignments(&mut self, assignments: &[Assignment]) {
        for (idx, assignment) in assignments.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, assignment.field.db);
            self.sql.push_str(" = ");
            self.render_assignment_value(&assignment.value);
        }
    }

    pub(super) fn render_assignment_value(&mut self, value: &AssignmentValue) {
        match value {
            AssignmentValue::Expr(expr) => self.render_value(expr),
            AssignmentValue::Default => {
                self.sql.push_str("DEFAULT");
            }
        }
    }

    pub(super) fn render_returning(&mut self, returning: &[SelectItem]) {
        if returning.is_empty() {
            return;
        }
        self.sql.push_str(" RETURNING ");
        for (idx, item) in returning.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_select_item(item);
        }
    }

    pub(super) fn render_conflict(&mut self, conflict: &ConflictClause) {
        self.sql.push_str(" ON CONFLICT ");
        match &conflict.target {
            ConflictTarget::Columns { fields, predicate } => {
                self.render_parenthesized_idents(fields.iter().map(|field| field.db));
                if let Some(predicate) = predicate {
                    self.sql.push_str(" WHERE ");
                    self.render_bool(predicate);
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
            }
            ConflictAction::DoUpdate {
                assignments,
                filter,
            } => {
                self.sql.push_str(" DO UPDATE SET ");
                self.render_assignments(assignments);
                if let Some(filter) = filter {
                    self.sql.push_str(" WHERE ");
                    self.render_bool(filter);
                }
            }
        }
    }

    fn render_parenthesized_idents<'a>(&mut self, idents: impl IntoIterator<Item = &'a str>) {
        self.sql.push('(');
        for (idx, ident) in idents.into_iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, ident);
        }
        self.sql.push(')');
    }
}
