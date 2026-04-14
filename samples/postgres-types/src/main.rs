use rqb::prelude::*;
use rqb_sample_base::schema::pg_type_examples;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
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

    let rows = select(pg_type_examples::dataset())
        .filter(pg_type_examples::DISPLAY_NAME.eq("ada"))
        .fetch_all_as::<PgTypeRow>(&db)
        .await?;

    println!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}
