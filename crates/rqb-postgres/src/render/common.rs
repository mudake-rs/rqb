use rqb_core::{FieldType, RawSql, ResolvedField, Source, ValidatedWriteValue, Value};

use crate::helpers::{
    postgres_cast_sql, postgres_selection_cast, renumber_postgres_placeholders, write_quoted_ident,
    write_quoted_qualified,
};
use crate::{Error, Result};

use super::Renderer;

impl Renderer {
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

    pub(super) fn render_source(&mut self, source: &Source) {
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
                self.sql.push('(');
                self.sql.push_str(sql);
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
        }
    }

    pub(super) fn render_write_target(&mut self, source: &Source) {
        match source {
            Source::Table { schema, name, .. } | Source::View { schema, name, .. } => {
                if let Some(schema) = schema {
                    write_quoted_ident(&mut self.sql, schema);
                    self.sql.push('.');
                }
                write_quoted_ident(&mut self.sql, name);
            }
            Source::Cte { name, .. } => write_quoted_ident(&mut self.sql, name),
            Source::Raw { alias, .. } => write_quoted_ident(&mut self.sql, alias),
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
            ValidatedWriteValue::Value(value) => self.push_typed_param(value, field_type),
            ValidatedWriteValue::Raw(raw) => self.render_raw(raw)?,
            ValidatedWriteValue::Column(field) => self.render_column_name(field),
        }
        Ok(())
    }

    pub(super) fn render_returning(&mut self, fields: &[ResolvedField]) {
        if fields.is_empty() {
            return;
        }
        self.sql.push_str(" RETURNING ");
        for (idx, field) in fields.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            self.render_selected_field(field);
        }
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

    pub(super) fn render_raw(&mut self, raw: &RawSql) -> Result<()> {
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
            let Some(value) = raw.binds.get(bind_index) else {
                return Err(Error::TooFewRawBinds);
            };
            bind_index += 1;
            self.push_param(value);
        }
        if bind_index != raw.binds.len() {
            return Err(Error::UnusedRawBinds);
        }
        Ok(())
    }

    pub(super) fn append_sql_with_params(&mut self, sql: &str, params: Vec<Value>) {
        let offset = self.params.len();
        self.sql
            .push_str(&renumber_postgres_placeholders(sql, offset));
        self.params.extend(params);
    }

    pub(super) fn push_param(&mut self, value: &Value) {
        self.params.push(value.clone());
        self.sql.push('$');
        self.sql.push_str(&self.params.len().to_string());
    }

    pub(super) fn push_typed_param(&mut self, value: &Value, field_type: FieldType) {
        self.push_param(value);
        if let Some(cast) = postgres_cast_sql(field_type) {
            self.sql.push_str(&cast);
        }
    }
}
