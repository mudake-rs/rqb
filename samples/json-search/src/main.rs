use rqb::prelude::*;
use rqb_sample_base::{ACME_ORG_ID, schema::order_search_view as order_search};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = rqb_sample_base::connect().await?;

    let request: SearchRequest = serde_json::from_str(
        r#"{
            "fields": ["id", "email", "status", "totalCents"],
            "filter": {
                "logical": "and",
                "predicates": [
                    { "field": "status", "operator": "equals", "value": "paid" },
                    { "field": "metadata.score", "operator": "gte", "value": 80 }
                ]
            },
            "sort": [{ "field": "totalCents", "dir": "desc" }],
            "limit": 10
        }"#,
    )?;

    let page = select(order_search::dataset())
        .filter(order_search::ORGANIZATION_ID.eq(ACME_ORG_ID))
        .request(request)
        .page_as::<serde_json::Value>(&db)
        .await?;

    println!("{}", serde_json::to_string_pretty(&page.items).unwrap());
    println!(
        "total={}, limit={}, offset={}",
        page.total, page.limit, page.offset
    );
    Ok(())
}
