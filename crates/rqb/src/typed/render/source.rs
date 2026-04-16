use super::*;

impl Renderer {
    pub(super) fn render_ctes(&mut self, ctes: &[Cte]) -> Result<()> {
        if ctes.is_empty() {
            return Ok(());
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
            if !cte.columns.is_empty() {
                self.render_cte_columns(cte.columns.iter().map(String::as_str));
            } else if !cte.fields.is_empty() {
                self.render_cte_columns(cte.fields.iter().map(|field| field.db));
            }
            self.sql.push_str(" AS");
            if let Some(materialization) = cte.materialization {
                self.sql.push(' ');
                self.sql.push_str(materialization.as_sql());
            }
            self.sql.push_str(" (");
            self.render_stmt(&cte.stmt)?;
            self.sql.push(')');
        }
        self.sql.push(' ');
        Ok(())
    }

    fn render_cte_columns<'a>(&mut self, columns: impl IntoIterator<Item = &'a str>) {
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
    pub(super) fn render_source(&mut self, source: &Source) -> Result<()> {
        match source {
            Source::Table { name, alias, .. } | Source::View { name, alias, .. } => {
                write_quoted_qualified(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Cte { name, alias, .. } => {
                write_quoted_ident(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Subquery { stmt, alias, .. } => {
                self.sql.push('(');
                self.render_stmt(stmt)?;
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
                self.render_source_column_list(source);
            }
            Source::Raw {
                sql, alias, params, ..
            } => {
                self.cacheable = false;
                self.sql.push('(');
                self.render_raw(sql, params)?;
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
                self.render_source_column_list(source);
            }
            Source::Function {
                name,
                args,
                alias,
                ordinality,
                fields,
                ..
            } => {
                self.render_call(name, args)?;
                if *ordinality {
                    self.sql.push_str(" WITH ORDINALITY");
                }
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
                if !fields.is_empty() {
                    self.sql.push_str(" (");
                    for (idx, field) in fields.iter().enumerate() {
                        if idx > 0 {
                            self.sql.push_str(", ");
                        }
                        write_quoted_ident(&mut self.sql, field.db);
                    }
                    self.sql.push(')');
                }
            }
        }
        Ok(())
    }

    pub(super) fn render_join(&mut self, join: &crate::typed::Join) -> Result<()> {
        self.sql.push(' ');
        self.sql.push_str(join.kind.as_sql());
        self.sql.push(' ');
        if join.lateral {
            self.sql.push_str("LATERAL ");
        }
        self.render_source(&join.source)?;
        if let Some(on) = &join.on {
            self.sql.push_str(" ON ");
            self.render_bool(on)?;
        }
        Ok(())
    }

    pub(super) fn render_optional_alias(&mut self, alias: Option<&str>) {
        if let Some(alias) = alias {
            self.sql.push_str(" AS ");
            write_quoted_ident(&mut self.sql, alias);
        }
    }

    fn render_source_column_list(&mut self, source: &Source) {
        match source {
            Source::Subquery { fields, .. } | Source::Raw { fields, .. } if !fields.is_empty() => {
                self.render_cte_columns(fields.iter().map(|field| field.db));
            }
            _ => {}
        }
    }

    pub(super) fn render_write_target(&mut self, source: &Source) {
        match source {
            Source::Table { name, alias, .. } | Source::View { name, alias, .. } => {
                write_quoted_qualified(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Cte { name, alias, .. } => {
                write_quoted_ident(&mut self.sql, name);
                self.render_optional_alias(alias.as_deref());
            }
            Source::Subquery { .. } | Source::Raw { .. } | Source::Function { .. } => {
                unreachable!("write target validated as table")
            }
        }
    }
    pub(super) fn render_field(&mut self, field: &crate::typed::Meta, qualifier: Option<&str>) {
        if let Some(qualifier) = qualifier {
            write_quoted_ident(&mut self.sql, qualifier);
            self.sql.push('.');
        }
        write_quoted_ident(&mut self.sql, field.db);
    }
}
