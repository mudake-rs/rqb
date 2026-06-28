use rqb::prelude::*;
use rqb_sample_schema::{InvoiceState, invoices};
use sqlx::postgres::PgPoolOptions;

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for generated enum integration tests");
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to Postgres sample database")
}

#[tokio::test]
#[ignore = "requires sample schema loaded in DATABASE_URL"]
async fn generated_enum_binds_and_decodes_with_sqlx() {
    let pool = pool().await;

    let decoded = rqb::raw("SELECT ?::sample.invoice_state")
        .bind(InvoiceState::Paid)
        .fetch_one_scalar::<InvoiceState>(&pool)
        .await
        .unwrap();
    assert_eq!(decoded, InvoiceState::Paid);

    let filtered = select(invoices::table())
        .expr(invoices::STATE)
        .filter(invoices::STATE.eq(InvoiceState::Paid))
        .limit(0)
        .fetch_scalar::<InvoiceState>(&pool)
        .await
        .unwrap();
    assert!(filtered.is_empty());
}
