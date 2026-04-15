use std::fmt;

use sqlx::postgres::PgArguments;
use sqlx::{Arguments, Encode, Postgres, Type};

use crate::{Error, Result};

/// Cloneable, type-erased Postgres bind parameter.
///
/// This is the existential boundary: `Param` stores some `T` that sqlx can
/// encode for Postgres. Generic `T` does not leak into the query AST.
pub trait ErasedParam: Send + Sync {
    fn clone_box(&self) -> Box<dyn ErasedParam>;
    fn add_to(&self, args: &mut PgArguments) -> Result<()>;
    fn debug_name(&self) -> &'static str;
}

impl Clone for Box<dyn ErasedParam> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct Param {
    inner: Box<dyn ErasedParam>,
}

impl Param {
    pub fn typed<T>(value: T) -> Self
    where
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        Self {
            inner: Box::new(TypedParam { value }),
        }
    }

    pub fn add_to(&self, args: &mut PgArguments) -> Result<()> {
        self.inner.add_to(args)
    }

    pub fn debug_name(&self) -> &'static str {
        self.inner.debug_name()
    }
}

impl Clone for Param {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl fmt::Debug for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Param").field(&self.debug_name()).finish()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Params {
    params: Vec<Param>,
}

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_vec(params: Vec<Param>) -> Self {
        Self { params }
    }

    pub fn push(&mut self, param: Param) {
        self.params.push(param);
    }

    pub fn push_typed<T>(&mut self, value: T)
    where
        T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        self.push(Param::typed(value));
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn as_slice(&self) -> &[Param] {
        &self.params
    }

    pub fn debug_names(&self) -> Vec<&'static str> {
        self.params.iter().map(Param::debug_name).collect()
    }

    pub fn arguments(&self) -> Result<PgArguments> {
        let mut args = PgArguments::default();
        for param in &self.params {
            param.add_to(&mut args)?;
        }
        Ok(args)
    }
}

struct TypedParam<T> {
    value: T,
}

impl<T> ErasedParam for TypedParam<T>
where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>,
{
    fn clone_box(&self) -> Box<dyn ErasedParam> {
        Box::new(Self {
            value: self.value.clone(),
        })
    }

    fn add_to(&self, args: &mut PgArguments) -> Result<()> {
        args.add(self.value.clone())
            .map_err(|error| Error::Encode(error.to_string()))
    }

    fn debug_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}
