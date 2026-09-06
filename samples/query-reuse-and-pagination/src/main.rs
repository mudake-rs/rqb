use chrono::{DateTime, Utc};
use rqb::dsl::row;
use rqb::prelude::*;
use rqb_sample_schema::{order_search_view, orders};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorSearchRequest {
    #[serde(default)]
    filter: Option<SearchFilter>,
}

#[derive(Clone, Copy, Debug)]
struct OrderCursor {
    created_at: DateTime<Utc>,
    id: Uuid,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let user_id = Uuid::nil();
    let cursor = OrderCursor {
        created_at: DateTime::parse_from_rfc3339("2026-06-28T12:00:00Z")?.with_timezone(&Utc),
        id: Uuid::parse_str("018f1f2a-7b9c-7d80-8000-000000000001")?,
    };

    let first_page = page_desc(user_id, None, 50).build()?;
    assert_eq!(
        first_page.sql,
        "SELECT \"id\", \"user_id\", \"status\", \"total_cents\", \"metadata\", \"tags\", \"created_at\" FROM \"sample\".\"orders\" WHERE \"user_id\" = $1 ORDER BY \"created_at\" DESC, \"id\" DESC LIMIT $2"
    );
    assert_eq!(first_page.params.len(), 2);

    let next_page = page_desc(user_id, Some(cursor), 50).build()?;
    assert_eq!(
        next_page.sql,
        "SELECT \"id\", \"user_id\", \"status\", \"total_cents\", \"metadata\", \"tags\", \"created_at\" FROM \"sample\".\"orders\" WHERE (\"user_id\" = $1 AND ROW(\"created_at\", \"id\") < ROW($2, $3)) ORDER BY \"created_at\" DESC, \"id\" DESC LIMIT $4"
    );
    assert_eq!(next_page.params.len(), 4);

    let asc_page = page_asc(user_id, Some(cursor), 25).build()?;
    assert_eq!(
        asc_page.sql,
        "SELECT \"id\", \"user_id\", \"status\", \"total_cents\", \"metadata\", \"tags\", \"created_at\" FROM \"sample\".\"orders\" WHERE (\"user_id\" = $1 AND ROW(\"created_at\", \"id\") > ROW($2, $3)) ORDER BY \"created_at\" ASC, \"id\" ASC LIMIT $4"
    );
    assert_eq!(asc_page.params.len(), 4);

    let uuidv7_page = page_by_id(user_id, Some(cursor.id), 50).build()?;
    assert_eq!(
        uuidv7_page.sql,
        "SELECT \"id\", \"user_id\", \"status\", \"total_cents\", \"metadata\", \"tags\", \"created_at\" FROM \"sample\".\"orders\" WHERE (\"user_id\" = $1 AND \"id\" < $2) ORDER BY \"id\" DESC LIMIT $3"
    );
    assert_eq!(uuidv7_page.params.len(), 3);

    let reusable = reusable_built_query(user_id)?;
    assert_eq!(
        reusable.sql,
        "SELECT \"id\", \"status\" FROM \"sample\".\"orders\" WHERE \"user_id\" = $1 ORDER BY \"created_at\" DESC LIMIT $2"
    );
    assert_eq!(reusable.params.len(), 2);
    assert!(reusable.summary().contains("Cacheable: true"));

    let offset_request: SearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "paid" },
        "sort": [{ "field": "created_at", "dir": "desc" }],
        "limit": 20,
        "offset": 40
    }))?;
    let offset_search = json_search_offset(user_id, offset_request)?.build()?;
    assert_eq!(
        offset_search.sql,
        "SELECT \"id\", \"user_id\", \"organization_id\", \"organization_slug\", \"user_email\", \"status\", \"total_cents\", \"tags\", \"metadata\", \"created_at\", \"item_count\", \"event_count\", \"last_event_at\" FROM \"sample\".\"order_search_view\" WHERE (\"user_id\" = $1 AND \"status\" = $2) ORDER BY \"created_at\" DESC LIMIT $3 OFFSET $4"
    );
    assert_eq!(offset_search.params.len(), 4);

    let filter_request: CursorSearchRequest = serde_json::from_value(json!({
        "filter": { "field": "status", "operator": "equals", "value": "paid" }
    }))?;
    let filtered_cursor_page =
        json_filter_then_server_cursor(user_id, filter_request, Some(cursor), 50)?.build()?;
    assert_eq!(
        filtered_cursor_page.sql,
        "SELECT \"id\", \"user_id\", \"organization_id\", \"organization_slug\", \"user_email\", \"status\", \"total_cents\", \"tags\", \"metadata\", \"created_at\", \"item_count\", \"event_count\", \"last_event_at\" FROM \"sample\".\"order_search_view\" WHERE (\"user_id\" = $1 AND \"status\" = $2 AND ROW(\"created_at\", \"id\") < ROW($3, $4)) ORDER BY \"created_at\" DESC, \"id\" DESC LIMIT $5"
    );
    assert_eq!(filtered_cursor_page.params.len(), 5);

    println!("{}", next_page.sql);
    println!("{}", offset_search.sql);
    println!("{}", filtered_cursor_page.sql);
    Ok(())
}

fn user_orders(user_id: Uuid) -> Select {
    // The generated schema already exposes the table fields. `select(table())`
    // uses that metadata for the default root projection; callers add only the
    // extra shape they own, such as filters, order, and cursor predicates.
    select(orders::table()).filter(orders::USER_ID.eq(user_id))
}

fn page_desc(user_id: Uuid, cursor: Option<OrderCursor>, limit: u32) -> Select {
    // Cursor pagination reuses the base query shape. Fetching one extra row
    // lets the application compute `has_next` without a matching count query.
    user_orders(user_id)
        .filter_option(cursor, |cursor| {
            row((orders::CREATED_AT, orders::ID)).lt((cursor.created_at, cursor.id))
        })
        .order_desc(orders::CREATED_AT)
        .order_desc(orders::ID)
        .limit(limit + 1)
}

fn page_asc(user_id: Uuid, cursor: Option<OrderCursor>, limit: u32) -> Select {
    // The seek predicate and ORDER BY direction must move together:
    // ASC uses `>` where the DESC version uses `<`.
    user_orders(user_id)
        .filter_option(cursor, |cursor| {
            row((orders::CREATED_AT, orders::ID)).gt((cursor.created_at, cursor.id))
        })
        .order_asc(orders::CREATED_AT)
        .order_asc(orders::ID)
        .limit(limit + 1)
}

fn page_by_id(user_id: Uuid, after_id: Option<Uuid>, limit: u32) -> Select {
    // If IDs are generated as UUIDv7 and id order is the desired order, the id
    // itself can be the cursor. UUIDv7 order tracks generation time, not commit
    // order, so use a stronger cursor for guaranteed sweeps under concurrency.
    user_orders(user_id)
        .filter_option(after_id, |id| orders::ID.lt(id))
        .order_desc(orders::ID)
        .limit(limit + 1)
}

fn reusable_built_query(user_id: Uuid) -> rqb::Result<BuiltQuery> {
    // Query-shape helpers return builders for further composition. Build once
    // when the application wants to log, inspect, and execute the same
    // validated SQL with the same bound params more than once. Rebuild for
    // different values.
    user_orders(user_id)
        .columns((orders::ID, orders::STATUS))
        .order_desc(orders::CREATED_AT)
        .limit(20)
        .build()
}

fn order_search() -> Select {
    select(order_search_view::view())
}

fn json_search_offset(user_id: Uuid, request: SearchRequest) -> rqb::Result<Select> {
    // Full JSON search owns filter, sort, limit, and offset. It is useful for
    // offset-style endpoints, but it is not a cursor API.
    order_search()
        .filter(order_search_view::USER_ID.eq(user_id))
        .apply_search(request)
}

fn json_filter_then_server_cursor(
    user_id: Uuid,
    request: CursorSearchRequest,
    cursor: Option<OrderCursor>,
    limit: u32,
) -> rqb::Result<Select> {
    // Cursor endpoints should accept a filter-only DTO, so client sort, limit,
    // and offset cannot be silently ignored by this server-owned page shape.
    let query = order_search().filter(order_search_view::USER_ID.eq(user_id));
    let query = match request.filter {
        Some(filter) => query.apply_filter(filter)?,
        None => query,
    };

    Ok(query
        .filter_option(cursor, |cursor| {
            row((order_search_view::CREATED_AT, order_search_view::ID))
                .lt((cursor.created_at, cursor.id))
        })
        .order_desc(order_search_view::CREATED_AT)
        .order_desc(order_search_view::ID)
        .limit(limit + 1))
}
