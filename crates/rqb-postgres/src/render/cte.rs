use rqb_core::{ValidatedCteBody, ValidatedSelect};

use crate::Result;
use crate::helpers::write_quoted_ident;

use super::Renderer;

impl Renderer {
    pub(super) fn render_ctes(&mut self, validated: &ValidatedSelect) -> Result<()> {
        if validated.ctes.is_empty() {
            return Ok(());
        }

        let recursive = validated.ctes.iter().any(|cte| cte.recursive);
        self.sql.push_str(if recursive {
            "WITH RECURSIVE "
        } else {
            "WITH "
        });

        for (idx, cte) in validated.ctes.iter().enumerate() {
            if idx > 0 {
                self.sql.push_str(", ");
            }
            write_quoted_ident(&mut self.sql, &cte.name);
            if !cte.columns.is_empty() {
                self.sql.push_str(" (");
                for (col_idx, column) in cte.columns.iter().enumerate() {
                    if col_idx > 0 {
                        self.sql.push_str(", ");
                    }
                    write_quoted_ident(&mut self.sql, column);
                }
                self.sql.push(')');
            }
            self.sql.push_str(" AS (");
            match &cte.body {
                ValidatedCteBody::Raw(raw) => self.render_raw(raw),
                ValidatedCteBody::Query(query) => {
                    self.render_query_expr(query, super::SelectProjection::Value, true)?
                }
            }
            self.sql.push(')');
        }
        self.sql.push(' ');
        Ok(())
    }
}
