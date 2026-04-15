use rqb::prelude::*;
use rqb_sample_base::schema::pg_type_examples;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
struct PgTypeRow {
    id: Uuid,
    display_name: String,
    payload: Vec<u8>,
    ip_addr: String,
    network: String,
    active_window: String,
    local_window: String,
    billing_dates: String,
    created_local: chrono::NaiveDateTime,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    // Generated metadata carries Postgres-specific casts for bytea, network, range,
    // timestamp, and timestamptz fields. Rust rows can use chrono types for
    // Postgres date/timestamp values.
    let rows = select(pg_type_examples::dataset())
        .fields([
            pg_type_examples::ID.into(),
            pg_type_examples::DISPLAY_NAME.alias("display_name"),
            pg_type_examples::PAYLOAD.into(),
            pg_type_examples::IP_ADDR.alias("ip_addr"),
            pg_type_examples::NETWORK.into(),
            pg_type_examples::ACTIVE_WINDOW.alias("active_window"),
            pg_type_examples::LOCAL_WINDOW.alias("local_window"),
            pg_type_examples::BILLING_DATES.alias("billing_dates"),
            pg_type_examples::CREATED_LOCAL.alias("created_local"),
            pg_type_examples::CREATED_AT.alias("created_at"),
        ])
        .filter(pg_type_examples::DISPLAY_NAME.eq("ada"))
        .fetch_all_as::<PgTypeRow>(&db)
        .await?;

    println!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}
