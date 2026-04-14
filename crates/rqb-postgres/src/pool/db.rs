use deadpool_postgres::{GenericClient, Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::{
    NoTls, Socket,
    tls::{MakeTlsConnect, TlsConnect},
    types::Type,
};

use crate::{Error, Result};

use super::{BeginBuilder, Tx, TxFuture};

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

    pub fn clear_statement_cache(&self) {
        self.pool.manager().statement_caches.clear();
    }

    pub fn remove_cached_statement(&self, query: &str, types: &[Type]) {
        self.pool.manager().statement_caches.remove(query, types);
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
        BeginBuilder::new(self)
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

fn pool_error(error: impl std::fmt::Display) -> Error {
    Error::Pool(error.to_string())
}
