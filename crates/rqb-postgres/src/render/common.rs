use rqb_core::{
    FieldType, RawSql, ResolvedField, Source, ValidatedReturningItem, ValidatedSelectItem,
    ValidatedSource, ValidatedWriteValue, Value,
};

use crate::helpers::{value_to_json, write_quoted_ident, write_quoted_qualified};
use crate::type_sql::postgres_selection_cast;
use crate::{Error, Result};

use super::Renderer;

impl Renderer {
    pub(crate) fn render_raw_query(mut self, raw: &RawSql) -> crate::BuiltQuery {
        self.render_raw(raw);
        self.finish()
    }

    pub(super) fn render_selected_field(&mut self, field: &ResolvedField) {
        self.render_column_name(field);
        let selection_cast = postgres_selection_cast(field.ty);
        if let Some(cast) = selection_cast {
            self.sql.push_str(cast);
        }
        if selection_cast.is_some()
            || field.explicit_qualifier.is_some()
            || field.alias.is_some()
            || field.api_name != field.db_name
        {
            self.sql.push_str(" AS ");
            write_quoted_ident(&mut self.sql, &field.output_alias());
        }
    }

    pub(super) fn render_selected_expr(&mut self, item: &ValidatedSelectItem) -> Result<()> {
        self.render_sql_expr(&item.expr)?;
        if let Some(cast) = postgres_selection_cast(item.ty) {
            self.sql.push_str(cast);
        }
        self.sql.push_str(" AS ");
        write_quoted_ident(&mut self.sql, &item.alias);
        Ok(())
    }

    pub(super) fn render_validated_source(&mut self, source: &ValidatedSource) -> Result<()> {
        match source {
            ValidatedSource::Plain(source) => self.render_source(source),
            ValidatedSource::Subquery { query, alias } => {
                self.sql.push('(');
                self.render_query_source(query)?;
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
                Ok(())
            }
        }
    }

    pub(super) fn render_source(&mut self, source: &Source) -> Result<()> {
        match source {
            Source::Table {
                schema,
                name,
                alias,
            }
            | Source::View {
                schema,
                name,
                alias,
            } => {
                if let Some(schema) = schema {
                    write_quoted_ident(&mut self.sql, schema);
                    self.sql.push('.');
                }
                write_quoted_ident(&mut self.sql, name);
                if let Some(alias) = alias {
                    self.sql.push_str(" AS ");
                    write_quoted_ident(&mut self.sql, alias);
                }
            }
            Source::Cte { name, alias } => {
                write_quoted_ident(&mut self.sql, name);
                if let Some(alias) = alias {
                    self.sql.push_str(" AS ");
                    write_quoted_ident(&mut self.sql, alias);
                }
            }
            Source::Raw { sql, alias } => {
                self.cacheable = false;
                self.sql.push('(');
                self.sql.push_str(sql);
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            Source::Subquery { .. } => {
                return Err(Error::Core(rqb_core::Error::UnsupportedWriteSource));
            }
        }
        Ok(())
    }

    pub(super) fn render_write_target(&mut self, source: &Source) {
        match source {
            Source::Table {
                schema,
                name,
                alias,
            }
            | Source::View {
                schema,
                name,
                alias,
            } => {
                if let Some(schema) = schema {
                    write_quoted_ident(&mut self.sql, schema);
                    self.sql.push('.');
                }
                write_quoted_ident(&mut self.sql, name);
                if let Some(alias) = alias {
                    self.sql.push_str(" AS ");
                    write_quoted_ident(&mut self.sql, alias);
                }
            }
            Source::Cte { name, alias } => {
                write_quoted_ident(&mut self.sql, name);
                if let Some(alias) = alias {
                    self.sql.push_str(" AS ");
                    write_quoted_ident(&mut self.sql, alias);
                }
            }
            Source::Raw { alias, .. } | Source::Subquery { alias, .. } => {
                self.cacheable = false;
                write_quoted_ident(&mut self.sql, alias);
            }
        }
    }

    pub(super) fn render_insert_columns(&mut self, fields: &[ResolvedField]) {
        self.sql.push_str(" (");
        for (idx, field) in fields.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, &field.db_name);
        }
        self.sql.push(')');
    }

    pub(super) fn render_write_value(
        &mut self,
        value: &ValidatedWriteValue,
        field_type: FieldType,
    ) -> Result<()> {
        match value {
            ValidatedWriteValue::Value(value) if field_type.is_jsonb() && !value.is_null() => {
                self.push_typed_param(&value_to_json(value), field_type)
            }
            ValidatedWriteValue::Value(value) => self.push_typed_param(value, field_type),
            ValidatedWriteValue::Raw(raw) => self.render_raw(raw),
            ValidatedWriteValue::Column(field) => self.render_column_name(field),
            ValidatedWriteValue::Expr(expr) => {
                self.sql.push_str("CAST(");
                self.render_sql_expr(expr)?;
                self.sql.push_str(" AS ");
                crate::type_sql::write_postgres_type_name(&mut self.sql, field_type);
                self.sql.push(')');
            }
            ValidatedWriteValue::Default => self.sql.push_str("DEFAULT"),
        }
        Ok(())
    }

    pub(super) fn render_returning(&mut self, items: &[ValidatedReturningItem]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        self.sql.push_str(" RETURNING ");
        for (idx, item) in items.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            match item {
                ValidatedReturningItem::Field(field) => self.render_selected_field(field),
                ValidatedReturningItem::Expression(item) => self.render_selected_expr(item)?,
            }
        }
        Ok(())
    }

    pub(super) fn render_column_name(&mut self, field: &ResolvedField) {
        if let Some(qualifier) = &field.qualifier {
            write_quoted_ident(&mut self.sql, qualifier);
            self.sql.push('.');
            write_quoted_ident(&mut self.sql, &field.db_name);
        } else {
            write_quoted_qualified(&mut self.sql, &field.db_name);
        }
    }

    pub(super) fn render_json_path(&mut self, path: &[String]) {
        self.sql.push_str("ARRAY[");
        for (idx, segment) in path.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.push_param(&Value::String(segment.clone()));
        }
        self.sql.push_str("]::text[]");
    }

    pub(super) fn render_raw(&mut self, raw: &RawSql) {
        self.cacheable = false;
        let mut bind_index = 0usize;
        let mut chars = raw.sql.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '?' {
                self.sql.push(ch);
                continue;
            }
            if chars.peek() == Some(&'?') {
                chars.next();
                self.sql.push('?');
                continue;
            }
            let value = raw
                .binds
                .get(bind_index)
                .expect("raw SQL bind count validated before rendering");
            bind_index += 1;
            self.push_param(value);
        }
        debug_assert_eq!(bind_index, raw.binds.len());
    }
}
