use rqb::prelude::*;
use rqb_sample_base::{ACME_ORG_ID, schema::order_search_view as order_search};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderSearchRow {
    id: String,
    email: String,
    status: rqb_sample_base::OrderStatus,
    total_cents: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let request: SearchRequest = serde_json::from_str(
        r#"{
            "filter": {
                "and": [
                    { "field": "status", "operator": "equals", "value": "paid" },
                    { "field": "metadata.score", "operator": "gte", "value": 80 }
                ]
            },
            "sort": [{ "field": "totalCents", "dir": "desc" }],
            "limit": 10
        }"#,
    )?;

    let page = select(order_search::dataset())
        .fields([
            order_search::ID,
            order_search::EMAIL,
            order_search::STATUS,
            order_search::TOTAL_CENTS,
        ])
        .filter(order_search::ORGANIZATION_ID.eq(ACME_ORG_ID))
        .request(request)
        .page_as::<OrderSearchRow>(&db)
        .await?;

    println!("{}", serde_json::to_string_pretty(&page.items).unwrap());
    println!(
        "total={}, limit={}, offset={}",
        page.total, page.limit, page.offset
    );
    Ok(())
}
