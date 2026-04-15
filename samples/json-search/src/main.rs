use rqb::prelude::*;
use serde_json::json;

mod orders {
    use rqb::prelude::*;

    pub static ID_META: Meta = Meta::new("id", "id", "uuid")
        .ops(OpSet::ordered())
        .json(JsonKind::Uuid);
    pub static ORGANIZATION_ID_META: Meta =
        Meta::new("organization_id", "organization_id", "uuid").ops(OpSet::equality());
    pub static STATUS_META: Meta = Meta::new("status", "status", "text")
        .ops(OpSet::ordered())
        .json(JsonKind::Text);
    pub static TOTAL_CENTS_META: Meta = Meta::new("total_cents", "total_cents", "int8")
        .ops(OpSet::ordered())
        .json(JsonKind::BigInt);
    pub static CREATED_AT_META: Meta = Meta::new("created_at", "created_at", "timestamptz")
        .ops(OpSet::ordered())
        .json(JsonKind::Timestamptz);

    pub const ID: Field<rqb::uuid::Uuid> = Field::new(&ID_META);
    pub const ORGANIZATION_ID: Field<rqb::uuid::Uuid> = Field::new(&ORGANIZATION_ID_META);
    pub const STATUS: Field<String> = Field::new(&STATUS_META);
    pub const TOTAL_CENTS: Field<i64> = Field::new(&TOTAL_CENTS_META);
    pub static FIELDS: [&Meta; 5] = [
        &ID_META,
        &ORGANIZATION_ID_META,
        &STATUS_META,
        &TOTAL_CENTS_META,
        &CREATED_AT_META,
    ];

    pub fn search_view() -> Source {
        rqb::view("public.order_search", &FIELDS)
    }
}

fn main() -> rqb::Result<()> {
    let current_org = rqb::uuid::Uuid::nil();
    let request: SearchRequest = serde_json::from_value(json!({
        "filter": {
            "and": [
                { "field": "status", "operator": "equals", "value": "paid" },
                { "field": "total_cents", "operator": "gte", "value": 5000 }
            ]
        },
        "sort": [{ "field": "created_at", "dir": "desc" }],
        "limit": 20,
        "offset": 0
    }))
    .unwrap();

    let built = select(orders::search_view())
        .column(orders::ID)
        .column(orders::STATUS)
        .column(orders::TOTAL_CENTS)
        .filter(orders::ORGANIZATION_ID.eq(current_org))
        .request(request)?
        .build()?;

    assert_eq!(
        built.sql,
        "SELECT \"id\", \"status\", \"total_cents\" FROM \"public\".\"order_search\" WHERE (\"organization_id\" = $1 AND (\"status\" = $2 AND \"total_cents\" >= $3)) ORDER BY \"created_at\" DESC LIMIT $4 OFFSET $5"
    );

    println!("{}", built.sql);
    Ok(())
}
