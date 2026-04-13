mod db;
mod executor;
mod transaction;

pub use db::{Db, connect, connect_with_tls};
pub use transaction::{BeginBuilder, IsolationLevel, Savepoint, Tx, TxFuture};

#[macro_export]
macro_rules! txn {
    (|$tx:ident| $body:block) => {
        |$tx| ::std::boxed::Box::pin(async move $body)
    };
}
