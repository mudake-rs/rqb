use rqb_core::Value;
use tokio_postgres::types::ToSql;

use crate::BindParam;

pub struct PgParams {
    inner: Vec<BindParam>,
}

impl PgParams {
    pub fn from_binds(values: &[BindParam]) -> Self {
        Self {
            inner: values.to_vec(),
        }
    }

    pub fn from_values(values: &[Value]) -> Self {
        Self {
            inner: values.iter().map(BindParam::from_value).collect(),
        }
    }

    pub fn as_refs(&self) -> Vec<&(dyn ToSql + Sync)> {
        bind_refs(&self.inner)
    }
}

pub(crate) fn bind_refs(values: &[BindParam]) -> Vec<&(dyn ToSql + Sync)> {
    values.iter().map(bind_ref).collect()
}

fn bind_ref(value: &BindParam) -> &(dyn ToSql + Sync) {
    value
}
