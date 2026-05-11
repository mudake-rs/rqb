use std::{fmt, sync::Arc};

use sqlx::postgres::PgArguments;
use sqlx::{Arguments, Encode, Postgres, Type};

use crate::{Error, Result};

/// Rust value that can be stored as a Postgres bind parameter.
///
/// This hides sqlx's encode/type bounds behind the rqb concept used by field
/// predicates, assignments, and raw parameters.
pub trait BindValue:
    Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>
{
}

impl<T> BindValue for T where
    T: Clone + Send + Sync + 'static + for<'q> Encode<'q, Postgres> + Type<Postgres>
{
}

/// Cloneable, type-erased Postgres bind parameter.
///
/// This is the existential boundary: `Param` stores some `T` that sqlx can
/// encode for Postgres. Generic `T` does not leak into the query AST.
trait ErasedParam: Send + Sync {
    fn add_to(&self, args: &mut PgArguments) -> Result<()>;
    fn debug_name(&self) -> &'static str;
}

/// Type-erased value that can be bound to Postgres through sqlx.
#[derive(Clone)]
#[must_use]
pub struct Param {
    // Arc keeps Param::clone() to a refcount bump instead of a heap-allocating dyn clone.
    inner: Arc<dyn ErasedParam>,
}

impl Param {
    /// Stores a typed value for later insertion into `PgArguments`.
    pub fn typed<T>(value: T) -> Self
    where
        T: BindValue,
    {
        Self {
            inner: Arc::new(TypedParam { value }),
        }
    }

    /// Adds this value to an existing sqlx `PgArguments` buffer.
    #[inline]
    pub fn add_to(&self, args: &mut PgArguments) -> Result<()> {
        self.inner.add_to(args)
    }

    /// Returns the stored Rust type name for diagnostics and tests.
    #[inline]
    pub fn debug_name(&self) -> &'static str {
        self.inner.debug_name()
    }
}

impl fmt::Debug for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Param").field(&self.debug_name()).finish()
    }
}

/// Ordered bind parameters for a built query.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct Params {
    params: Vec<Param>,
}

impl Params {
    /// Creates an empty parameter list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a parameter list from an existing vector.
    #[inline]
    pub fn from_vec(params: Vec<Param>) -> Self {
        Self { params }
    }

    /// Appends an already-erased parameter.
    #[inline]
    pub fn push(&mut self, param: Param) {
        self.params.push(param);
    }

    /// Appends a typed value as a Postgres bind parameter.
    pub fn push_typed<T>(&mut self, value: T)
    where
        T: BindValue,
    {
        self.push(Param::typed(value));
    }

    /// Returns the number of parameters.
    #[inline]
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Returns true when there are no parameters.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Returns the parameters as a slice in SQL placeholder order.
    #[inline]
    pub fn as_slice(&self) -> &[Param] {
        &self.params
    }

    /// Returns stored Rust type names in parameter order.
    pub fn debug_names(&self) -> Vec<&'static str> {
        self.params.iter().map(Param::debug_name).collect()
    }

    /// Converts the stored values into sqlx Postgres arguments.
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
    T: BindValue,
{
    fn add_to(&self, args: &mut PgArguments) -> Result<()> {
        args.add(self.value.clone())
            .map_err(|error| Error::Encode(error.to_string()))
    }

    fn debug_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::{Param, Params};

    #[test]
    fn typed_param_reports_the_stored_rust_type_name() {
        let param = Param::typed(42_i32);

        assert_eq!(param.debug_name(), std::any::type_name::<i32>());
    }

    #[test]
    fn cloned_param_preserves_the_erased_type() {
        let param = Param::typed("paid".to_owned());
        let cloned = param.clone();

        assert_eq!(param.debug_name(), cloned.debug_name());
    }

    #[test]
    fn params_preserve_insertion_order_for_debug_names() {
        let mut params = Params::new();
        assert!(params.is_empty());

        params.push_typed(1_i32);
        params.push_typed("two".to_owned());
        params.push_typed(true);

        assert!(!params.is_empty());
        assert_eq!(params.as_slice().len(), 3);
        assert_eq!(
            params.debug_names(),
            vec![
                std::any::type_name::<i32>(),
                std::any::type_name::<String>(),
                std::any::type_name::<bool>(),
            ]
        );
    }

    #[test]
    fn params_arguments_accept_all_stored_values() {
        let params = Params::from_vec(vec![
            Param::typed(1_i32),
            Param::typed("two".to_owned()),
            Param::typed(false),
        ]);

        params.arguments().unwrap();
    }

    #[test]
    fn param_debug_prints_the_erased_rust_type_name() {
        let param = Param::typed(1_i64);

        assert_eq!(
            format!("{param:?}"),
            format!("Param({:?})", std::any::type_name::<i64>())
        );
    }
}
