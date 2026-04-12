use rqb_core::{CteBody, ValidatedSelect};

use crate::helpers::write_quoted_ident;
use crate::{Postgres, Result};

use super::Renderer;

impl Renderer {
    pub(super) fn render_ctes(&mut self, validated: &ValidatedSelect) -> Result<()> {
        if validated.query.ctes.is_empty() {
            return Ok(());
        }

        let recursive = validated.query.ctes.iter().any(|cte| cte.recursive);
        self.sql.push_str(if recursive {
            "WITH RECURSIVE "
        } else {
            "WITH "
        });

        for (idx, cte) in validated.query.ctes.iter().enumerate() {
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
                CteBody::Raw(raw) => self.render_raw(raw)?,
                CteBody::Select(query) => {
                    let built = Postgres::build_rows((**query).clone())?;
                    self.append_sql_with_params(&built.sql, built.params);
                }
            }
            self.sql.push(')');
        }
        self.sql.push(' ');
        Ok(())
    }
}
