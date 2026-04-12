use rqb_core::{Expr, ValidatedAggregate, ValidatedSelect};

use crate::Result;
use crate::helpers::{postgres_selection_cast, quote_literal, write_quoted_ident};

use super::Renderer;

impl Renderer {
    pub(super) fn render_aggregate(
        &mut self,
        validated: &ValidatedSelect,
        aggregate: &ValidatedAggregate,
    ) -> Result<()> {
        match aggregate {
            ValidatedAggregate::Count { alias, filter } => {
                self.sql.push_str("COUNT(*)");
                self.render_aggregate_filter(validated, filter)?;
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::CountField {
                field,
                alias,
                distinct,
                filter,
            } => {
                self.sql.push_str("COUNT(");
                if *distinct {
                    self.sql.push_str("DISTINCT ");
                }
                self.render_column_name(field);
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::Sum {
                field,
                alias,
                filter,
            } => {
                if filter.is_some() {
                    self.sql.push('(');
                }
                self.sql.push_str("SUM(");
                self.render_column_name(field);
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                if filter.is_some() {
                    self.sql.push(')');
                }
                self.sql.push_str("::double precision AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::Avg {
                field,
                alias,
                filter,
            } => {
                if filter.is_some() {
                    self.sql.push('(');
                }
                self.sql.push_str("AVG(");
                self.render_column_name(field);
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                if filter.is_some() {
                    self.sql.push(')');
                }
                self.sql.push_str("::double precision AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::Min {
                field,
                alias,
                filter,
            } => {
                let selection_cast = postgres_selection_cast(field.ty);
                let wrap_for_cast = filter.is_some() && selection_cast.is_some();
                if wrap_for_cast {
                    self.sql.push('(');
                }
                self.sql.push_str("MIN(");
                self.render_column_name(field);
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                if wrap_for_cast {
                    self.sql.push(')');
                }
                if let Some(cast) = selection_cast {
                    self.sql.push_str(cast);
                }
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::Max {
                field,
                alias,
                filter,
            } => {
                let selection_cast = postgres_selection_cast(field.ty);
                let wrap_for_cast = filter.is_some() && selection_cast.is_some();
                if wrap_for_cast {
                    self.sql.push('(');
                }
                self.sql.push_str("MAX(");
                self.render_column_name(field);
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                if wrap_for_cast {
                    self.sql.push(')');
                }
                if let Some(cast) = selection_cast {
                    self.sql.push_str(cast);
                }
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::JsonAgg {
                alias,
                fields,
                order_by,
                filter,
                default_empty,
            } => {
                if *default_empty {
                    self.sql.push_str("COALESCE(");
                }
                self.sql.push_str("jsonb_agg(jsonb_build_object(");
                for (idx, field) in fields.iter().enumerate() {
                    if idx > 0 {
                        self.sql.push_str(", ");
                    }
                    self.sql.push_str(&quote_literal(field.object_key()));
                    self.sql.push_str(", ");
                    self.render_column_name(field);
                    if let Some(cast) = postgres_selection_cast(field.ty) {
                        self.sql.push_str(cast);
                    }
                }
                self.sql.push(')');
                if let Some(sort) = order_by {
                    self.sql.push_str(" ORDER BY ");
                    self.render_column_name(&sort.field);
                    self.sql.push(' ');
                    self.sql.push_str(sort.dir.as_str());
                    if let Some(nulls) = sort.nulls {
                        self.sql.push(' ');
                        self.sql.push_str(nulls.as_str());
                    }
                }
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                if *default_empty {
                    self.sql.push_str(", '[]'::jsonb)");
                }
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::ArrayAgg {
                field,
                alias,
                distinct,
                order_by,
                filter,
            } => {
                self.sql.push_str("to_jsonb(array_agg(");
                if *distinct {
                    self.sql.push_str("DISTINCT ");
                }
                self.render_column_name(field);
                if let Some(cast) = postgres_selection_cast(field.ty) {
                    self.sql.push_str(cast);
                }
                if let Some(sort) = order_by {
                    self.sql.push_str(" ORDER BY ");
                    self.render_column_name(&sort.field);
                    self.sql.push(' ');
                    self.sql.push_str(sort.dir.as_str());
                    if let Some(nulls) = sort.nulls {
                        self.sql.push(' ');
                        self.sql.push_str(nulls.as_str());
                    }
                }
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                self.sql.push_str(") AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
            ValidatedAggregate::StringAgg {
                field,
                separator,
                alias,
                order_by,
                filter,
            } => {
                self.sql.push_str("string_agg(");
                self.render_column_name(field);
                self.sql.push_str(", ");
                self.sql.push_str(&quote_literal(separator));
                if let Some(sort) = order_by {
                    self.sql.push_str(" ORDER BY ");
                    self.render_column_name(&sort.field);
                    self.sql.push(' ');
                    self.sql.push_str(sort.dir.as_str());
                }
                self.sql.push(')');
                self.render_aggregate_filter(validated, filter)?;
                self.sql.push_str(" AS ");
                write_quoted_ident(&mut self.sql, alias);
            }
        }
        Ok(())
    }

    fn render_aggregate_filter(
        &mut self,
        validated: &ValidatedSelect,
        filter: &Option<Expr>,
    ) -> Result<()> {
        if let Some(filter) = filter {
            self.sql.push_str(" FILTER (WHERE ");
            self.render_expr(validated, filter)?;
            self.sql.push(')');
        }
        Ok(())
    }
}
