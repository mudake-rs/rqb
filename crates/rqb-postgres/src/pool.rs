use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

use deadpool_postgres::{GenericClient, Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::{
    NoTls, Row, Socket,
    tls::{MakeTlsConnect, TlsConnect},
    types::ToSql,
};

use crate::helpers::quote_ident;
use crate::{Error, PgExecutor, Result};

pub type TxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

#[macro_export]
macro_rules! txn {
    (|$tx:ident| $body:block) => {
        |$tx| ::std::boxed::Box::pin(async move $body)
    };
}

#[derive(Clone)]
pub struct Db {
    pool: Pool,
}

impl Db {
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_max_size(url, 16).await
    }

    pub async fn connect_with_max_size(url: &str, max_size: usize) -> Result<Self> {
        Self::connect_with_max_size_and_tls(url, max_size, NoTls).await
    }

    pub async fn connect_with_tls<T>(url: &str, tls: T) -> Result<Self>
    where
        T: MakeTlsConnect<Socket> + Clone + Sync + Send + 'static,
        T::Stream: Sync + Send,
        T::TlsConnect: Sync + Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        Self::connect_with_max_size_and_tls(url, 16, tls).await
    }

    pub async fn connect_with_max_size_and_tls<T>(
        url: &str,
        max_size: usize,
        tls: T,
    ) -> Result<Self>
    where
        T: MakeTlsConnect<Socket> + Clone + Sync + Send + 'static,
        T::Stream: Sync + Send,
        T::TlsConnect: Sync + Send,
        <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        let pg_config = url.parse::<tokio_postgres::Config>().map_err(Error::from)?;
        let manager_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let manager = Manager::from_config(pg_config, tls, manager_config);
        let pool = Pool::builder(manager)
            .max_size(max_size)
            .build()
            .map_err(pool_error)?;
        let db = Self { pool };
        db.ping().await?;
        Ok(db)
    }

    pub fn from_pool(pool: Pool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub async fn get(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.map_err(pool_error)
    }

    pub async fn ping(&self) -> Result<()> {
        let client = self.get().await?;
        GenericClient::batch_execute(&client, "SELECT 1")
            .await
            .map_err(Error::from)?;
        Ok(())
    }

    pub fn begin(&self) -> BeginBuilder<'_> {
        BeginBuilder {
            db: self,
            isolation: None,
            read_only: false,
            deferrable: false,
        }
    }

    pub async fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        T: Send,
        F: for<'tx> FnOnce(&'tx Tx) -> TxFuture<'tx, T> + Send,
    {
        self.transaction_with(|builder| builder, f).await
    }

    pub async fn transaction_with<T, F>(
        &self,
        config: impl FnOnce(BeginBuilder<'_>) -> BeginBuilder<'_>,
        f: F,
    ) -> Result<T>
    where
        T: Send,
        F: for<'tx> FnOnce(&'tx Tx) -> TxFuture<'tx, T> + Send,
    {
        let tx = config(self.begin()).await?;
        match f(&tx).await {
            Ok(value) => {
                tx.commit().await?;
                Ok(value)
            }
            Err(error) => {
                let _ = tx.rollback().await;
                Err(error)
            }
        }
    }
}

pub async fn connect(url: &str) -> Result<Db> {
    Db::connect(url).await
}

pub async fn connect_with_tls<T>(url: &str, tls: T) -> Result<Db>
where
    T: MakeTlsConnect<Socket> + Clone + Sync + Send + 'static,
    T::Stream: Sync + Send,
    T::TlsConnect: Sync + Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    Db::connect_with_tls(url, tls).await
}

impl PgExecutor for Db {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let client = self.get().await?;
        GenericClient::query(&client, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let client = self.get().await?;
        GenericClient::query_opt(&client, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        let client = self.get().await?;
        GenericClient::query_opt(&client, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        GenericClient::execute(&client, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let client = self.get().await?;
        query_cached(&client, sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let client = self.get().await?;
        query_one_cached(&client, sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        let client = self.get().await?;
        query_opt_cached(&client, sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        execute_cached(&client, sql, params).await
    }
}

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

    fn client(&self) -> Result<&deadpool_postgres::Client> {
        self.client
            .as_ref()
            .ok_or_else(|| Error::Connection("transaction is already closed".to_owned()))
    }
}

impl PgExecutor for Tx {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        GenericClient::query(self.client()?, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        GenericClient::query_opt(self.client()?, sql, params)
            .await
            .map_err(Error::from)?
            .ok_or(Error::NotFound)
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        GenericClient::query_opt(self.client()?, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        GenericClient::execute(self.client()?, sql, params)
            .await
            .map_err(Error::from)
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        query_cached(self.client()?, sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        query_one_cached(self.client()?, sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        query_opt_cached(self.client()?, sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        execute_cached(self.client()?, sql, params).await
    }
}

async fn query_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Row>> {
    let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
    client.query(&stmt, params).await.map_err(Error::from)
}

async fn query_one_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Row> {
    query_opt_cached(client, sql, params)
        .await?
        .ok_or(Error::NotFound)
}

async fn query_opt_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Option<Row>> {
    let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
    client.query_opt(&stmt, params).await.map_err(Error::from)
}

async fn execute_cached(
    client: &(impl GenericClient + ?Sized),
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64> {
    let stmt = client.prepare_cached(sql).await.map_err(Error::from)?;
    client.execute(&stmt, params).await.map_err(Error::from)
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
}

impl PgExecutor for Savepoint<'_> {
    async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        self.tx.query(sql, params).await
    }

    async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        self.tx.query_one(sql, params).await
    }

    async fn query_opt(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        self.tx.query_opt(sql, params).await
    }

    async fn execute_sql(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        self.tx.execute_sql(sql, params).await
    }

    async fn query_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        self.tx.query_cached(sql, params).await
    }

    async fn query_one_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        self.tx.query_one_cached(sql, params).await
    }

    async fn query_opt_cached(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        self.tx.query_opt_cached(sql, params).await
    }

    async fn execute_sql_cached(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        self.tx.execute_sql_cached(sql, params).await
    }
}

fn pool_error(error: impl std::fmt::Display) -> Error {
    Error::Pool(error.to_string())
}
