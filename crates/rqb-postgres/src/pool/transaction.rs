use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

use deadpool_postgres::GenericClient;

use crate::helpers::quote_ident;
use crate::{Error, Result};

use super::Db;

pub type TxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    fn as_sql(self) -> &'static str {
        match self {
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        }
    }
}

#[derive(Clone, Copy)]
#[must_use]
pub struct BeginBuilder<'a> {
    db: &'a Db,
    isolation: Option<IsolationLevel>,
    read_only: bool,
    deferrable: bool,
}

impl<'a> BeginBuilder<'a> {
    pub(super) fn new(db: &'a Db) -> Self {
        Self {
            db,
            isolation: None,
            read_only: false,
            deferrable: false,
        }
    }

    pub fn isolation(mut self, level: IsolationLevel) -> Self {
        self.isolation = Some(level);
        self
    }

    pub fn read_committed(self) -> Self {
        self.isolation(IsolationLevel::ReadCommitted)
    }

    pub fn repeatable_read(self) -> Self {
        self.isolation(IsolationLevel::RepeatableRead)
    }

    pub fn serializable(self) -> Self {
        self.isolation(IsolationLevel::Serializable)
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn deferrable(mut self) -> Self {
        self.deferrable = true;
        self
    }

    pub async fn start(self) -> Result<Tx> {
        let client = self.db.get().await?;
        GenericClient::batch_execute(&client, &self.begin_sql())
            .await
            .map_err(Error::from)?;
        Ok(Tx {
            client: Some(client),
            done: false,
        })
    }

    fn begin_sql(self) -> String {
        let mut sql = String::from("BEGIN");
        if let Some(isolation) = self.isolation {
            sql.push_str(" ISOLATION LEVEL ");
            sql.push_str(isolation.as_sql());
        }
        if self.read_only {
            sql.push_str(" READ ONLY");
        }
        if self.deferrable {
            sql.push_str(" DEFERRABLE");
        }
        sql
    }
}

impl<'a> IntoFuture for BeginBuilder<'a> {
    type Output = Result<Tx>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.start())
    }
}

#[must_use = "transaction must be committed or explicitly rolled back"]
pub struct Tx {
    client: Option<deadpool_postgres::Client>,
    done: bool,
}

impl Tx {
    pub async fn commit(mut self) -> Result<()> {
        GenericClient::batch_execute(self.client()?, "COMMIT")
            .await
            .map_err(Error::from)?;
        self.done = true;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<()> {
        GenericClient::batch_execute(self.client()?, "ROLLBACK")
            .await
            .map_err(Error::from)?;
        self.done = true;
        Ok(())
    }

    pub async fn savepoint(&self, name: impl Into<String>) -> Result<Savepoint<'_>> {
        let name = name.into();
        GenericClient::batch_execute(self.client()?, &format!("SAVEPOINT {}", quote_ident(&name)))
            .await
            .map_err(Error::from)?;
        Ok(Savepoint { tx: self, name })
    }

    pub(super) fn client(&self) -> Result<&deadpool_postgres::Client> {
        self.client
            .as_ref()
            .ok_or_else(|| Error::Connection("transaction is already closed".to_owned()))
    }
}

impl Drop for Tx {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        let Some(client) = self.client.take() else {
            return;
        };
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        handle.spawn(async move {
            let _ = GenericClient::batch_execute(&client, "ROLLBACK").await;
        });
    }
}

#[must_use]
pub struct Savepoint<'a> {
    tx: &'a Tx,
    name: String,
}

impl Savepoint<'_> {
    pub async fn release(self) -> Result<()> {
        GenericClient::batch_execute(
            self.tx.client()?,
            &format!("RELEASE SAVEPOINT {}", quote_ident(&self.name)),
        )
        .await
        .map_err(Error::from)?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        GenericClient::batch_execute(
            self.tx.client()?,
            &format!("ROLLBACK TO SAVEPOINT {}", quote_ident(&self.name)),
        )
        .await
        .map_err(Error::from)?;
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn tx(&self) -> &Tx {
        self.tx
    }
}
