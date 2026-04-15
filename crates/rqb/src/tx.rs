/// Run an async move block in a sqlx transaction.
#[macro_export]
macro_rules! tx {
    ($pool:expr, |$conn:ident| $body:block $(,)?) => {
        async move {
            let mut __rqb_tx = $pool.begin().await.map_err($crate::Error::from)?;
            let __rqb_result: $crate::Result<_> = {
                let $conn = &mut *__rqb_tx;
                (async move $body).await
            };

            match __rqb_result {
                Ok(__rqb_value) => {
                    __rqb_tx.commit().await.map_err($crate::Error::from)?;
                    Ok(__rqb_value)
                }
                Err(__rqb_error) => {
                    match __rqb_tx.rollback().await {
                        Ok(()) => Err(__rqb_error),
                        Err(__rqb_rollback) => Err($crate::Error::TransactionRollbackFailed {
                            error: Box::new(__rqb_error),
                            rollback: Box::new($crate::Error::from(__rqb_rollback)),
                        }),
                    }
                }
            }
        }
    };
}
