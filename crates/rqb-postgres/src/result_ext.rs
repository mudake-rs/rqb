use crate::{Error, Result};

pub trait ResultExt<T> {
    fn optional(self) -> Result<Option<T>>;

    fn on_conflict<E>(self, f: impl FnOnce(&Error) -> E) -> std::result::Result<T, E>
    where
        E: From<Error>;

    fn on_constraint<E>(self, name: &str, f: impl FnOnce(&Error) -> E) -> std::result::Result<T, E>
    where
        E: From<Error>;
}

impl<T> ResultExt<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(Error::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn on_conflict<E>(self, f: impl FnOnce(&Error) -> E) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(ref error) if error.is_unique_violation() => Err(f(error)),
            Err(error) => Err(E::from(error)),
        }
    }

    fn on_constraint<E>(self, name: &str, f: impl FnOnce(&Error) -> E) -> std::result::Result<T, E>
    where
        E: From<Error>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(ref error) if error.is_constraint(name) => Err(f(error)),
            Err(error) => Err(E::from(error)),
        }
    }
}
