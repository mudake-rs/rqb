# Recipes

## Search API Endpoint

```rust
use actix_web::{HttpResponse, web};
use rqb::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Page<T> {
    items: Vec<T>,
    total: i64,
    limit: u32,
    offset: u64,
}

async fn search_orders(
    db: web::Data<Db>,
    body: web::Json<SearchRequest>,
) -> Result<HttpResponse, AppError> {
    let page = select(order_search())
        .request(body.into_inner())
        .page_as::<serde_json::Value>(&**db)
        .await?;
    Ok(HttpResponse::Ok().json(Page {
        items: page.items,
        total: page.total,
        limit: page.limit,
        offset: page.offset,
    }))
}
```

The HTTP body becomes `SearchRequest`, and the same validation path protects fields, operators, limits, and sort order.

## Search Over A Server-Owned Query Shape

Use this when the client needs dynamic filtering but must not choose joins or raw SQL.

```rust
let users = users().alias("u");
let orders = orders().alias("o");

let page = select(users)
    .left_join(orders, ID.on("u").eq_col(USER_ID.on("o")))
    .filter(ORGANIZATION_ID.on("u").eq(current_org_id))
    .request(body.into_inner())
    .page_as::<serde_json::Value>(&db)
    .await?;
```

The JSON request can refer to fields in the joined scope, for example `"u.email"` or `"o.status"`, but the join itself remains server-owned Rust code.

## Pagination With Total Count

```rust
let page = select(order_search())
    .fields([ID, EMAIL, STATUS, CREATED_AT])
    .filter(STATUS.eq("paid"))
    .order_by(CREATED_AT.desc())
    .limit(params.limit.unwrap_or(20))
    .offset(params.offset.unwrap_or(0))
    .page_as::<OrderRow>(&db)
    .await?;
```

`page_as` executes the rows query and matching count query concurrently through the same executor, then returns rows, total, limit, and offset.

## Debug Generated SQL

```rust
let built = select(order_search())
    .filter(STATUS.eq("paid"))
    .order_by(CREATED_AT.desc())
    .build_pg()?;

tracing::debug!("{}", built.rows.debug_sql());
tracing::debug!("{}", built.count.debug_sql());
```

`debug_sql()` includes bind params as `Value` debug output. Keep it for diagnostics; do not paste params into SQL strings.

## Partial Update From JSON Body

```rust
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchOrder {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<OrderMetadata>,
}

let updated = update(orders())
    .set_from(&patch)
    .filter(ID.eq(order_id))
    .fetch_optional_as::<Order>(&db)
    .await?;
```

Serde removes absent fields before `set_from` maps the remaining keys to known fields. Without `skip_serializing_if`, `None` is serialized as SQL `NULL`, which is useful for explicit nulling but wrong for most patch DTOs.
Use `.set_null(field)` for explicit SQL `NULL` assignments.

## Upsert Create Or Update

```rust
insert(users())
    .value(&new_user)
    .on_conflict(EMAIL)
    .do_update([STATUS, PROFILE, TAGS])
    .fetch_one_as::<UserRow>(&db)
    .await?;
```

Use `.do_nothing()` when a duplicate is acceptable and no update should run.

## Nested Objects Without N+1

```rust
#[derive(serde::Deserialize)]
struct UserWithOrders {
    id: uuid::Uuid,
    email: String,
    orders: Vec<OrderSummary>,
}

let rows = select(users().alias("u"))
    .left_join(orders().alias("o"), USER_ID.on("o").eq_col(ID.on("u")))
    .fields([ID.on("u"), EMAIL.on("u")])
    .json_agg("orders", [ID.on("o"), STATUS.on("o"), CREATED_AT.on("o")])
    .filter_agg("orders", ID.on("o").is_not_null())
    .fetch_as::<UserWithOrders>(&db)
    .await?;
```

Root fields deserialize with clean names. Joined flat fields keep a prefix, and `json_agg` returns `[]` instead of `null`.

## Bulk Insert With Returning

```rust
let rows = insert(order_items())
    .values(&items)
    .returning([ID, ORDER_ID, PRODUCT_ID])
    .fetch_as::<OrderItemRow>(&db)
    .await?;
```

Batch inserts use one `INSERT ... VALUES (...), (...)` query.

## Conditional Filters

```rust
let query = select(order_search())
    .filter_option(params.status, |status| STATUS.eq(status))
    .filter_option(params.channel, |channel| CHANNEL.eq(channel));
```

This keeps optional request parameters out of `match` boilerplate.

## Correlated EXISTS

```rust
let rows = select(orders().alias("o"))
    .filter(exists(
        select(events().alias("e")).filter(all([
            EVENT_ORDER_ID.on("e").eq_col(ID.on("o")),
            EVENT_TYPE.on("e").eq("paid"),
        ])),
    ))
    .fetch_as::<OrderRow>(&db)
    .await?;
```

The inner query can reference outer aliases. Use this for "has related row" filters without joining and duplicating root rows.

## IN Subquery

```rust
let rows = select(users())
    .filter(ID.in_subquery(
        select(orders().alias("o"))
            .fields([USER_ID.on("o")])
            .filter(STATUS.on("o").eq("paid")),
    ))
    .fetch_as::<UserRow>(&db)
    .await?;
```

The subquery must select exactly one column. rqb does not apply the dataset default limit inside predicate subqueries.

## JSONB Field Queries

```rust
select(order_search())
    .filter(all([
        METADATA.path("score").gte(80),
        METADATA.key_exists("campaign"),
        METADATA.path("gift").eq(true),
    ]));
```

The field must opt in with `.json_paths(JsonPathPolicy::Dynamic)`. Key existence operators work on top-level JSONB fields, not paths.

## Soft Delete Pattern

```rust
update(orders())
    .set_raw(field("deleted_at"), raw("now()"))
    .filter(ID.eq(order_id))
    .returning([ID])
    .fetch_optional(&db)
    .await?;
```

For generated schemas, prefer a real `DELETED_AT` field constant over `field("deleted_at")`.

## Multi-Tenant Filtering

```rust
fn tenant_orders(tenant_id: uuid::Uuid) -> SelectBuilder {
    select(order_search()).filter(ORGANIZATION_ID.eq(tenant_id))
}

let rows = tenant_orders(current_org)
    .and_where(STATUS.eq("paid"))
    .fetch_as::<OrderRow>(&db)
    .await?;
```

Put the required tenant predicate in a small constructor and only expose that constructor to handlers.

## Archive And Delete In One Query

```rust
let deleted = cte(
    "deleted",
    raw("DELETE FROM orders WHERE created_at < now() - interval '1 year' RETURNING *"),
);

insert(Dataset::table("orders_archive").fields([ID, USER_ID, STATUS, CREATED_AT]))
    .from_select(
        select(Dataset::cte("deleted").fields([ID, USER_ID, STATUS, CREATED_AT]))
            .cte(deleted)
            .fields([ID, USER_ID, STATUS, CREATED_AT])
            .build(),
    )
    .execute(&db)
    .await?;
```

Use raw CTEs for SQL shapes that are outside the current builder surface.

## Latest Row Per Group

Postgres `DISTINCT ON` is useful for one row per grouping key.

```rust
let latest = select(orders())
    .fields([USER_ID, ID, STATUS, CREATED_AT])
    .distinct_on([USER_ID])
    .order_by(USER_ID.asc())
    .order_by(CREATED_AT.desc())
    .fetch_as::<OrderRow>(&db)
    .await?;
```

The `ORDER BY` should start with the same fields used in `DISTINCT ON`.

## Work Queue With SKIP LOCKED

```rust
let tx = db.begin().await?;

let jobs = select(job_queue())
    .filter(STATUS.eq("ready"))
    .order_by(CREATED_AT.asc())
    .limit(100)
    .for_update()
    .skip_locked()
    .fetch_as::<Job>(&tx)
    .await?;

for job in &jobs {
    update(job_queue())
        .set(STATUS, "running")
        .filter(ID.eq(job.id))
        .execute(&tx)
        .await?;
}

tx.commit().await?;
```

Use `FOR UPDATE SKIP LOCKED` inside a transaction when multiple workers claim rows from the same table.

## Transaction With Retry On Serialization Failure

```rust
async fn run_serializable(db: &Db) -> Result<(), rqb::postgres::Error> {
    for attempt in 0..3 {
        let result = async {
            let tx = db.begin().serializable().await?;
            update(accounts())
                .set_raw(BALANCE, raw("balance - 100"))
                .filter(ID.eq("source"))
                .execute(&tx)
                .await?;
            update(accounts())
                .set_raw(BALANCE, raw("balance + 100"))
                .filter(ID.eq("target"))
                .execute(&tx)
                .await?;
            tx.commit().await
        }
        .await;

        if result.is_ok() || attempt == 2 {
            return result;
        }
    }
    Ok(())
}
```

The current error type stores unknown database errors as `Database { code, .. }`; match SQLSTATE `40001` there if you need strict serialization-only retries.

## Raw SQL Escape Hatch

```rust
select(orders())
    .filter(raw("(metadata->>'risk')::int > ?").bind(50))
    .order_by(CREATED_AT.desc());

update(orders())
    .set_raw(METADATA, raw("metadata || jsonb_build_object('reviewed', true)"))
    .filter(ID.eq(order_id));
```

Raw SQL supports `?` binds and `??` for a literal `?`.

## Generated Schema Workflow

```bash
make db-up
cargo run -p rqb-cli -- generate \
  --database-url postgres://rqb:rqb@localhost:55432/rqb \
  --schema public \
  --out target/generated/rqb_schema.rs
```

```rust
mod schema {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/target/generated/rqb_schema.rs"));
}

let rows = select(schema::order_search_view::dataset())
    .fields([schema::order_search_view::ID, schema::order_search_view::EMAIL])
    .fetch_as::<serde_json::Value>(&db)
    .await?;
```
