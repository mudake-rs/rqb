use rqb_core::{
    DeleteBuilder, DeleteQuery, Error as CoreError, InsertBuilder, InsertQuery, RawQuery,
    SelectBuilder, SelectQuery, UpdateBuilder, UpdateQuery, ValidatedDelete, ValidatedInsert,
    ValidatedSelect, ValidatedUpdate,
};

use crate::render::Renderer;
use crate::{BuiltQuery, BuiltSelect, Error, Result};

pub struct Postgres;

impl Postgres {
    pub fn build(query: SelectQuery) -> Result<BuiltSelect> {
        let (built, _, _) = Self::build_page(query)?;
        Ok(built)
    }

    pub(crate) fn build_page(query: SelectQuery) -> Result<(BuiltSelect, u32, u64)> {
        let validated = ValidatedSelect::new(query)?;
        let limit = validated.limit;
        let offset = validated.offset;
        let built = BuiltSelect {
            rows: Renderer::new().render_rows(&validated)?,
            count: Renderer::new().render_count(&validated)?,
        };
        Ok((built, limit, offset))
    }

    pub fn build_rows(query: SelectQuery) -> Result<BuiltQuery> {
        let validated = ValidatedSelect::new(query)?;
        Renderer::new().render_rows(&validated)
    }

    pub fn build_insert(query: InsertQuery) -> Result<BuiltQuery> {
        let validated = ValidatedInsert::new(query)?;
        Renderer::new().render_insert(&validated)
    }

    pub fn build_update(query: UpdateQuery) -> Result<BuiltQuery> {
        let validated = ValidatedUpdate::new(query)?;
        Renderer::new().render_update(&validated)
    }

    pub fn build_delete(query: DeleteQuery) -> Result<BuiltQuery> {
        let validated = ValidatedDelete::new(query)?;
        Renderer::new().render_delete(&validated)
    }

    pub fn build_raw_query(query: RawQuery) -> Result<BuiltQuery> {
        let raw = query.as_raw_sql();
        let placeholders = raw.placeholder_count();
        if placeholders != raw.binds.len() {
            return Err(Error::Core(CoreError::RawBindMismatch {
                placeholders,
                binds: raw.binds.len(),
            }));
        }
        Ok(Renderer::new().render_raw_query(raw))
    }
}

pub trait BuildPostgres {
    type Output;

    fn build_pg(self) -> Result<Self::Output>;
}

impl BuildPostgres for SelectQuery {
    type Output = BuiltSelect;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build(self)
    }
}

pub trait BuildRowsPostgres {
    fn build_rows_pg(self) -> Result<BuiltQuery>;
}

impl BuildRowsPostgres for SelectQuery {
    fn build_rows_pg(self) -> Result<BuiltQuery> {
        Postgres::build_rows(self)
    }
}

impl BuildPostgres for SelectBuilder {
    type Output = BuiltSelect;

    fn build_pg(self) -> Result<Self::Output> {
        self.build().build_pg()
    }
}

impl BuildRowsPostgres for SelectBuilder {
    fn build_rows_pg(self) -> Result<BuiltQuery> {
        self.build().build_rows_pg()
    }
}

impl BuildPostgres for InsertQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_insert(self)
    }
}

impl BuildPostgres for InsertBuilder {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        self.build()?.build_pg()
    }
}

impl BuildPostgres for UpdateQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_update(self)
    }
}

impl BuildPostgres for UpdateBuilder {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        self.build()?.build_pg()
    }
}

impl BuildPostgres for DeleteQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_delete(self)
    }
}

impl BuildPostgres for DeleteBuilder {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        self.build().build_pg()
    }
}

impl BuildPostgres for RawQuery {
    type Output = BuiltQuery;

    fn build_pg(self) -> Result<Self::Output> {
        Postgres::build_raw_query(self)
    }
}
