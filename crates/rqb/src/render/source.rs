use super::*;

impl Renderer {
    pub(super) fn render_ctes(&mut self, ctes: &[Cte]) {
        if ctes.is_empty() {
            return;
        }
        self.sql.push_str(if ctes.iter().any(|cte| cte.recursive) {
            "WITH RECURSIVE "
        } else {
            "WITH "
        });
        for (idx, cte) in ctes.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, &cte.name);
            if !cte.fields.is_empty() {
                self.render_paren_column_list(cte.fields.iter().map(|field| field.db));
            }
            self.sql.push_str(" AS");
            if let Some(materialization) = cte.materialization {
                self.sql.push(' ');
                self.sql.push_str(materialization.as_sql());
            }
            self.sql.push_str(" (");
            self.render_stmt(&cte.stmt);
            self.sql.push(')');
        }
        self.sql.push(' ');
    }

    fn render_paren_column_list<'a>(&mut self, columns: impl IntoIterator<Item = &'a str>) {
        self.sql.push_str(" (");
        for (column_idx, column) in columns.into_iter().enumerate() {
            if column_idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, column);
        }
        self.sql.push(')');
    }

    pub(super) fn render_source_fields(&mut self, source: &Source) {
        let mut rendered = 0usize;
        let qualifier = source.explicit_alias();
        source.for_each_field(|field| {
            if rendered > 0 {
                self.sql.push_str(", ");
            }
            self.render_field(field, qualifier);
            if field.api != field.db {
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, field.api);
            }
            rendered += 1;
        });
        if rendered == 0 {
            self.sql.push('*');
        }
    }

    pub(super) fn render_source(&mut self, source: &Source) {
        match source {
            Source::Table { name, alias, .. } | Source::View { name, alias, .. } => {
                write_quoted_qualified(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Cte { name, alias, .. } => {
                write_quoted_ident(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Subquery {
                stmt,
                alias,
                fields,
            } => {
                self.sql.push('(');
                self.render_stmt(stmt);
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
                if !fields.is_empty() {
                    self.render_paren_column_list(fields.iter().map(|field| field.db));
                }
            }
            Source::Raw {
                sql,
                alias,
                params,
                fields,
            } => {
                self.sql.push('(');
                self.render_raw(sql, params);
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
                if !fields.is_empty() {
                    self.render_paren_column_list(fields.iter().map(|field| field.db));
                }
            }
            Source::Function {
                name,
                args,
                alias,
                ordinality,
                fields,
                ..
            } => {
                self.render_call(name, args);
                if *ordinality {
                    self.sql.push_str(" WITH ORDINALITY");
                }
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
                if !fields.is_empty() {
                    self.render_paren_column_list(fields.iter().map(|field| field.db));
                }
            }
            Source::Values {
                rows,
                alias,
                fields,
            } => {
                self.sql.push_str("(VALUES ");
                for (row_idx, row) in rows.iter().enumerate() {
                    if row_idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.sql.push('(');
                    for (value_idx, value) in row.iter().enumerate() {
                        if value_idx > 0 {
                            self.sql.push_str(", ");
                        }
                        self.render_value(value);
                    }
                    self.sql.push(')');
                }
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
                if !fields.is_empty() {
                    self.render_paren_column_list(fields.iter().map(|field| field.db));
                }
            }
        }
    }

    pub(super) fn render_join(&mut self, join: &crate::source::Join) {
        self.sql.push(' ');
        self.sql.push_str(join.kind.as_sql());
        self.sql.push(' ');
        if join.lateral {
            self.sql.push_str("LATERAL ");
        }
        self.render_source(&join.source);
        if let Some(on) = &join.on {
            self.sql.push_str(" ON ");
            self.render_bool(on);
        }
    }

    pub(super) fn render_optional_alias(&mut self, alias: Option<&str>) {
        if let Some(alias) = alias {
            self.sql.push_str(" AS ");
            write_quoted_ident(&mut self.sql, alias);
        }
    }

    pub(super) fn render_write_target(&mut self, source: &Source) {
        match source {
            Source::Table { name, alias, .. } | Source::View { name, alias, .. } => {
                write_quoted_qualified(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Cte { .. }
            | Source::Subquery { .. }
            | Source::Raw { .. }
            | Source::Function { .. }
            | Source::Values { .. } => {
                unreachable!("write target validated as table")
            }
        }
    }

    pub(super) fn render_field(&mut self, field: &crate::Meta, qualifier: Option<&str>) {
        if let Some(qualifier) = qualifier {
            write_quoted_ident(&mut self.sql, qualifier);
            self.sql.push('.');
        }
        write_quoted_ident(&mut self.sql, field.db);
    }
}
