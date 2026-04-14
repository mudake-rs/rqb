# Guide

## Datasets And Fields

Fields describe the API name, database column name, type, and allowed operations. `Field::new` is for identical API and DB names. `Field::mapped` is for camelCase APIs over snake_case columns.

```rust
use rqb::prelude::*;

pub const ORDER_STATUS: EnumType = EnumType::new(
    Some("public"),
    "order_status",
    &["draft", "paid", "cancelled", "refunded"],
);

pub const ID: Field = Field::new("id", FieldType::Uuid);
pub const USER_ID: Field = Field::mapped("userId", "user_id", FieldType::Uuid);
pub const STATUS: Field = Field::new("status", FieldType::Enum(ORDER_STATUS));
pub const STATUS_HISTORY: Field =
    Field::mapped("statusHistory", "status_history", FieldType::Array(ElemType::Enum(ORDER_STATUS)));
pub const TOTAL_CENTS: Field = Field::mapped("totalCents", "total_cents", FieldType::BigInt);
pub const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
    .sortable(false)
    .json_paths(JsonPathPolicy::Dynamic);
pub const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
pub const CREATED_AT: Field = Field::mapped("createdAt", "created_at", FieldType::Timestamptz);

pub fn orders() -> Dataset {
    Dataset::table("orders")
        .fields([ID, USER_ID, STATUS, STATUS_HISTORY, TOTAL_CENTS, METADATA, TAGS, CREATED_AT])
        .default_limit(50)
        .max_limit(500)
}
```

`selectable(false)`, `sortable(false)`, and `filterable(false)` are validation policy, not SQL generation hints. JSON paths are denied by default; enable them with `json_paths(JsonPathPolicy::Dynamic)`. Full text search is denied by default; enable it with `text_search("english")`.

Sources:

```rust
Dataset::table("orders");
Dataset::view("order_search_view");
Dataset::raw("SELECT * FROM orders WHERE archived_at IS NULL", "active_orders");
select(orders())
    .fields([ID, USER_ID])
    .into_source("order_ids")
    .fields([ID, USER_ID]);
Dataset::cte("recent_orders");
Dataset::table("orders").alias("o");
```

## Queries

### SELECT

```rust
let query = select(orders())
    .distinct()
    .fields([ID, STATUS, TOTAL_CENTS])
    .filter(all([
        STATUS.eq("paid"),
        any([TAGS.has("vip"), METADATA.path("gift").eq(true)]),
        not(TOTAL_CENTS.lt(1000)),
    ]))
    .order_by(CREATED_AT.desc().nulls_last())
    .limit(20)
    .offset(40);
```

`.fields(...)` is optional. When it is omitted, rqb selects all `selectable` fields from the root dataset:

```rust
let rows = select(orders())
    .filter(STATUS.eq("paid"))
    .fetch_all_as::<Order>(&db)
    .await?;
```

Use `.fields([...])` when the response should be narrower, when selecting qualified columns from a join, when building a one-column `IN (subquery)`, or when returning `serde_json::Value` from a client-selected field list.

Use `.select_expr(...)` for server-owned computed columns. Computed expressions
must be aliased because the alias is the serde/result field name:

```rust
#[derive(serde::Deserialize)]
struct OrderRow {
    id: uuid::Uuid,
    label: String,
    email_lower: String,
    created_day: chrono::DateTime<chrono::Utc>,
    status_label: String,
    total_text: String,
}

let rows = select(orders())
    .select([ID])
    .select_expr(coalesce([DISPLAY_NAME.expr(), EMAIL.expr()]).alias("label"))
    .select_expr(lower(EMAIL).alias("email_lower"))
    .select_expr(date_trunc("day", CREATED_AT).alias("created_day"))
    .select_expr(
        case_when(STATUS.eq("paid"))
            .then("settled")
            .otherwise("open")
            .alias("status_label"),
    )
    .select_expr(cast(TOTAL_CENTS.expr(), FieldType::Text).alias("total_text"))
    .fetch_all_as::<OrderRow>(&db)
    .await?;
```

`.select_expr(...)` is Rust-only query shape. JSON `SearchRequest` can still
select, sort, and filter only dataset-declared fields; it cannot reference a
computed alias such as `label` unless that alias is exposed through dataset
metadata, for example by a view.

`.filter(expr)` and `.and_where(expr)` AND-compose with the current filter. `.or_where(expr)` OR-composes with it. Use `.replace_filter(expr)` when replacement is intentional. `.filter_if(condition, expr)` is useful for already-normalized params. `.filter_option(value, |value| ...)` handles optional values without unwraps.

`.request(search_request)` merges JSON search input into the current builder. Server-side filters are preserved and combined with the request filter using `AND`; request fields, sort, limit, and offset replace those parts when present. Use `.replace_request(search_request)` only when replacement is intended.

```rust
let query = select(orders())
    .filter_option(params.status, |status| STATUS.eq(status))
    .filter_option(params.min_total, |min_total| TOTAL_CENTS.gte(min_total));
```

Postgres `DISTINCT ON`:

```rust
let latest_per_user = select(orders())
    .fields([USER_ID, ID, CREATED_AT, STATUS])
    .distinct_on([USER_ID])
    .order_by(USER_ID.asc())
    .order_by(CREATED_AT.desc());
```

Row locks:

```rust
let jobs = select(job_queue())
    .filter(STATUS.eq("ready"))
    .order_by(CREATED_AT.asc())
    .limit(100)
    .for_update()
    .skip_locked();
```

Lock modes: `.for_update()`, `.for_no_key_update()`, `.for_share()`, `.for_key_share()`. Wait modes: `.nowait()`, `.skip_locked()`.

### JSON Search Requests

`SearchRequest` is the serde-friendly request type used for JSON APIs:

```json
{
  "fields": ["id", "status", "totalCents"],
  "sort": [{ "field": "createdAt", "dir": "DESC" }],
  "query": {
    "logical": "and",
    "predicates": [
      { "field": "status", "operator": "equals", "value": "paid" },
      { "field": "metadata.score", "operator": "gte", "value": 80 }
    ]
  },
  "limit": 20,
  "offset": 0
}
```

Server code controls the query shape:

```rust
let page = select(order_search())
    .filter(ORGANIZATION_ID.eq(current_org_id))
    .request(request)
    .page_as::<serde_json::Value>(&db)
    .await?;
```

JSON can select fields, filter, sort, and page. It cannot define joins, CTEs, raw SQL, subqueries, aggregates, or transactions. Build those in Rust and then apply `.request(request)`.

### INSERT

```rust
insert(orders())
    .set(ID, order_id)
    .set(USER_ID, user_id)
    .set(STATUS, "draft")
    .fetch_one_as::<Order>(&db)
    .await?;
```

Write `fetch_*` methods return all selectable fields by default. Add `.returning([ID, STATUS])` when you want a narrower projection, or use `.execute()` when no rows should be returned.

`INSERT` also accepts server-owned expressions and SQL defaults:

```rust
insert(events())
    .set(ID, event_id)
    .set(EVENT_TYPE, "created")
    .set_default(PAYLOAD)
    .set_expr(CREATED_AT, now())
    .returning([ID])
    .returning_expr(lower(EVENT_TYPE).alias("kind"));
```

Insert expressions cannot reference target fields because Postgres `VALUES` rows do not have a current target row. Use `.set(...)`, `.set_default(...)`, functions, or server-owned raw expressions.

Struct inserts use serde:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NewOrder {
    id: uuid::Uuid,
    user_id: uuid::Uuid,
    status: String,
    metadata: serde_json::Value,
}

insert(orders()).value(&new_order);
insert(orders()).values(&new_orders);
```

Upsert:

```rust
insert(orders())
    .value(&new_order)
    .on_conflict(ID)
    .do_update([STATUS, METADATA])
    .conflict_filter(STATUS.ne("cancelled"));
```

For custom upsert assignments, pass write assignments explicitly. `excluded(...)`
is only valid inside `ON CONFLICT DO UPDATE` assignments:

```rust
insert(order_counters())
    .set(ID, id)
    .set(TOTAL_CENTS, total)
    .on_conflict(ID)
    .index_where(DELETED_AT.is_null())
    .do_update_set([
        set_expr(TOTAL_CENTS, excluded(TOTAL_CENTS)),
        set_default(UPDATED_AT),
    ])
    .conflict_filter(DELETED_AT.is_null())
    .returning([ID, TOTAL_CENTS]);
```

Insert from select:

```rust
let source = select(Dataset::table("orders_archive")).fields([ID, USER_ID, STATUS]).build();

insert(orders())
    .from_select(source)
    .returning([ID]);
```

### UPDATE

```rust
update(orders())
    .set(STATUS, "paid")
    .set_default(METADATA)
    .set_expr(TOTAL_CENTS, coalesce([TOTAL_CENTS.expr(), 0.into_sql_expr()]))
    .set_raw(CREATED_AT, raw("now()"))
    .set_col(USER_ID, field("backup_user_id"))
    .filter(ID.eq(order_id))
    .returning_expr(cast(TOTAL_CENTS.expr(), FieldType::Text).alias("total_text"))
    .fetch_optional_as::<Order>(&db)
    .await?;
```

Use `.set_null(field)` when an update needs to assign SQL `NULL` without spelling `Value::Null`. Use `.set_default(field)` for SQL `DEFAULT`. Use `.set_expr(field, expr)` for server-owned computed assignments; rqb validates the expression type and casts the top-level expression to the target field type.

`UPDATE ... FROM` adds extra datasets to the write scope:

```rust
update(orders().alias("o"))
    .from(users().alias("u"))
    .set_col(orders::USER_ID, users::ID.on("u"))
    .filter(orders::USER_ID.on("o").eq_col(users::ID.on("u")))
    .returning([orders::ID.on("o").alias("id")]);
```

Partial update:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchOrder {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

update(orders()).set_from(&patch).filter(ID.eq(order_id));
```

For patch DTOs, put `#[serde(skip_serializing_if = "Option::is_none")]` on optional fields. Otherwise `None` means an explicit SQL `NULL` assignment.

### DELETE

```rust
delete(orders())
    .filter(STATUS.eq("draft"))
    .returning([ID]);
```

`DELETE` without a filter is rejected during validation. Use an explicit predicate so broad deletes are visible in code review.

`DELETE ... USING` works the same way when the delete predicate needs another
source:

```rust
delete(events().alias("e"))
    .using(orders().alias("o"))
    .filter(events::ORDER_ID.on("e").eq_col(orders::ID.on("o")))
    .execute(&db)
    .await?;
```

## Joins

Use dataset aliases and qualified fields.

```rust
let users = Dataset::table("app_users").alias("u").fields([ID, EMAIL]);
let orders = Dataset::table("orders").alias("o").fields([ID, USER_ID, STATUS]);

let rows = select(users)
    .left_join(orders, ID.on("u").eq_col(USER_ID.on("o")))
    .fields([ID.on("u"), EMAIL.on("u"), STATUS.on("o")])
    .filter(STATUS.on("o").eq("paid"));
```

Available joins: `.join`, `.left_join`, `.right_join`, `.full_join`, `.cross_join`. For ad hoc qualified fields, `field("o.status")` is accepted, but typed constants plus `.on("o")` keep validation stronger.

If `.fields(...)` is omitted on a joined query, rqb selects the root dataset fields only. Joined datasets are in scope for filters, sorting, aggregates, and explicit projections, but they are not returned by default.

`SearchRequest` can be applied to a joined query. The JSON body still does not create the join; it can only refer to fields that the server-owned scope already exposes.

```rust
let query = select(users().alias("u"))
    .left_join(orders().alias("o"), ID.on("u").eq_col(USER_ID.on("o")))
    .request(request);
```

With aliases, request fields like `"u.email"` and `"o.status"` validate against the joined scope.

## CTEs

Raw CTE:

```rust
let paid = cte(
    "paid_orders",
    raw("SELECT id, user_id, status FROM orders WHERE status = ?").bind("paid"),
);

select(Dataset::cte("paid_orders").fields([ID, USER_ID, STATUS]))
    .cte(paid)
    .fields([ID, USER_ID]);
```

Nested select CTE:

```rust
let recent = select(orders())
    .fields([ID, USER_ID, STATUS])
    .filter(CREATED_AT.gte("2026-01-01T00:00:00Z"))
    .build();

select(Dataset::cte("recent_orders").fields([ID, USER_ID, STATUS]))
    .cte(cte("recent_orders", recent))
    .filter(STATUS.eq("paid"));
```

Recursive CTE:

```rust
select(Dataset::cte("tree").fields([ID]))
    .cte(cte("tree", raw("SELECT id FROM nodes WHERE parent_id IS NULL UNION ALL SELECT n.id FROM nodes n JOIN tree t ON n.parent_id = t.id")).recursive());
```

## Set Operations

`union`, `union_all`, `intersect`, and `except` compose full query bodies. rqb
validates that both sides select the same number of columns and that matching
columns have compatible output types.

```rust
let active = select(users())
    .fields([EMAIL])
    .filter(STATUS.eq("active"));

let disabled = select(users())
    .fields([EMAIL])
    .filter(STATUS.eq("disabled"));

let emails = union(active, disabled)
    .order_by(field("email").asc())
    .limit(100)
    .fetch_all_as::<EmailRow>(&db)
    .await?;
```

Set queries can be used anywhere rqb accepts a query body, including CTEs,
`EXISTS`, `IN (subquery)`, and `INSERT ... SELECT`:

```rust
let candidate_user_ids = union_all(
    select(orders().alias("paid"))
        .fields([USER_ID.on("paid")])
        .filter(STATUS.on("paid").eq("paid")),
    select(orders().alias("draft"))
        .fields([USER_ID.on("draft")])
        .filter(STATUS.on("draft").eq("draft")),
);

select(users())
    .filter(ID.in_subquery(candidate_user_ids));
```

## Subqueries

Correlated `EXISTS`:

```rust
let query = select(orders().alias("o"))
    .filter(exists(
        select(events().alias("e")).filter(all([
            EVENT_ORDER_ID.on("e").eq_col(ID.on("o")),
            EVENT_TYPE.on("e").eq("paid"),
        ])),
    ));
```

Negated `EXISTS`:

```rust
let query = select(orders().alias("o"))
    .filter(not_exists(
        select(events().alias("e"))
            .filter(EVENT_ORDER_ID.on("e").eq_col(ID.on("o"))),
    ));
```

`IN (subquery)` and `NOT IN (subquery)`:

```rust
let query = select(users())
    .filter(ID.in_subquery(
        select(orders().alias("o"))
            .fields([USER_ID.on("o")])
            .filter(STATUS.on("o").eq("paid")),
    ));
```

Subqueries are validated with access to the outer query scope, so correlated references like `ID.on("o")` work. `IN` subqueries must select exactly one column. Subquery expressions are Rust-only and skipped by serde; they are intentionally not part of the JSON request format.

### Subquery Sources And LATERAL

Use `into_source` when the source SQL should still be built and validated by
rqb. The declared fields describe the subquery output columns.

```rust
let paid_orders = select(orders().alias("o"))
    .fields([ID.on("o"), USER_ID.on("o")])
    .filter(STATUS.on("o").eq("paid"))
    .into_source("paid_orders")
    .fields([ID, USER_ID]);

select(paid_orders)
    .fields([USER_ID])
    .filter(USER_ID.eq(user_id));
```

Use `join_lateral`, `left_join_lateral`, or `cross_join_lateral` when the
subquery source references fields from the left side of the `FROM` list:

```rust
let latest_order = select(orders().alias("o"))
    .fields([STATUS.on("o")])
    .filter(USER_ID.on("o").eq_col(ID.on("u")))
    .order_by(CREATED_AT.on("o").desc())
    .limit(1)
    .into_source("latest_order")
    .fields([STATUS]);

select(users().alias("u"))
    .fields([EMAIL.on("u"), STATUS.on("latest_order").alias("latestStatus")])
    .left_join_lateral(latest_order, raw("TRUE"));
```

Non-lateral subquery sources are validated without access to outer fields, so an
accidental correlated reference fails before SQL rendering.

## Raw SQL

`raw("... ? ...").bind(value)` uses `?` placeholders and rqb bind values. Use `??` for a literal question mark.

```rust
select(orders()).filter(raw("metadata @? ?").bind("$.lines[*] ? (@.qty > 2)"));

update(orders())
    .set_raw(METADATA, raw("jsonb_set(metadata, '{reviewed}', 'true'::jsonb)"))
    .filter(ID.eq(order_id));
```

Raw fragments are an escape hatch. They are not introspected beyond bind counting.

Use `raw_query(...)` when the whole statement is outside the builder surface:

```rust
raw_query("CALL refresh_order_search(?)")
    .bind(order_id)
    .execute(&tx)
    .await?;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStats {
    status: String,
    orders: i64,
    avg_total_cents: f64,
}

let stats = raw_query(
    "SELECT status::text AS status, \
            COUNT(*)::bigint AS orders, \
            AVG(total_cents)::float8 AS \"avgTotalCents\" \
     FROM order_search_view \
     WHERE status = ?::text::order_status \
     GROUP BY status",
)
.bind("paid")
.fetch_all_as::<RawStats>(&db)
.await?;

let version: String = raw_query("SELECT version()")
    .fetch_one_scalar(&db)
    .await?;
```

`raw_query` is top-level SQL. It works with `&Db`, `&Tx`, and any `&impl PgExecutor`. `fetch_all_as` maps by returned column names, so alias expressions to match the target struct. It does not use dataset metadata; cast custom or ambiguous SQL expressions to the shape you want to deserialize. For example, cast exact numeric values to `text` when you need strings, or to `float8` only when lossy floating-point output is acceptable.

## Aggregations And GROUP BY

```rust
let stats = select(orders())
    .fields([STATUS])
    .agg(count("orders"))
    .agg(sum(TOTAL_CENTS, "totalCents"))
    .group_by([STATUS])
    .having(raw("SUM(total_cents) > ?").bind(0))
    .order_by(STATUS.asc());
```

Nested objects through `json_agg`:

```rust
let rows = select(Dataset::table("app_users").alias("u").fields([ID, EMAIL]))
    .left_join(Dataset::table("orders").alias("o").fields([ID, USER_ID, STATUS]), ID.on("u").eq_col(USER_ID.on("o")))
    .fields([ID.on("u"), EMAIL.on("u")])
    .agg(
        json_agg("orders", [ID.on("o"), STATUS.on("o")])
            .filter(ID.on("o").is_not_null())
    );
```

When aggregates are present and `.group_by(...)` is omitted, rqb groups by selected fields. Aggregates: `count`, `count_field`, `count_distinct`, `sum`, `avg`, `min`, `max`, `array_agg`, `json_agg`, and `string_agg`.

`json_agg` defaults to `[]` for empty aggregate results. Use `json_agg_nullable` when a SQL `NULL` result is part of the API contract.
Prefer inline aggregate modifiers such as `json_agg(...).filter(...)` and `array_agg(...).order_by(...)`. Alias-based modifiers such as `filter_agg` and `order_within` remain available and validate the aggregate alias; typos fail at build time instead of being ignored.

## Operators

| Method | SQL shape | Field types |
| --- | --- | --- |
| `eq`, `ne` | `=`, `<>` | scalar, JSONB, enum |
| `is_distinct_from`, `is_not_distinct_from` | null-safe equality | scalar, JSONB, enum |
| `gt`, `gte`, `lt`, `lte` | comparison | numeric, temporal, text, enum, numeric JSON paths |
| `between`, `not_between` | range comparison | numeric, temporal, text, enum, numeric JSON paths |
| `is_in`, `not_in` | `IN (...)`, `NOT IN (...)` | scalar, enum |
| `in_subquery`, `not_in_subquery` | `IN (SELECT ...)`, `NOT IN (SELECT ...)` | scalar, enum |
| `contains`, `not_contains`, `starts_with`, `ends_with` | `ILIKE`; range/network containment for `contains` | text, uuid, JSON paths, ranges, inet/cidr |
| `not_starts_with`, `not_ends_with` | negated `ILIKE` | text, uuid, JSON paths |
| `regex`, `not_regex` | `~*`, `!~*` | text, JSON paths |
| `contained_by`, `overlaps` | `<@` / `&&`; network uses `<<=` / `&&` | ranges, inet/cidr |
| `has`, `not_has` | element `= ANY(array)` | arrays |
| `contains_any`, `contains_all` | `&&`, `@>` | arrays |
| `is_empty`, `is_not_empty` | `cardinality(...)` | arrays |
| `elem_match` | JSONB/array containment | arrays, JSONB |
| `key_exists`, `keys_exist_any`, `keys_exist_all` | `?`, `?|`, `?&` | top-level JSONB |
| `search` | `to_tsvector @@ websearch_to_tsquery` | fields with `text_search(config)` |
| `is_null`, `is_not_null` | `IS NULL`, `IS NOT NULL` | any field |
| `eq_col`, `ne_col`, `gt_col`, `gte_col`, `lt_col`, `lte_col` | column-to-column compare | equal types or numeric-compatible |
| `exists`, `not_exists` | `EXISTS (SELECT ...)`, `NOT EXISTS (SELECT ...)` | expression-level helper |

JSON path comparisons use `#>` for JSON equality and `#>>` for text/numeric operators.

## Type Mapping

### Input

| Rust value | `Value` | SQL cast |
| --- | --- | --- |
| `&str`, `String` | `String` | text, uuid, timestamp, date, enum as needed |
| integer types | `I64` | bigint, int |
| `f32`, `f64` | `F64` | double precision, numeric |
| exact numeric strings | `String` | numeric and numeric-like domains |
| `Value::bytes(...)`, `&[u8]` | `Bytes` | bytea |
| `bool` | `Bool` | boolean |
| `serde_json::Value` | `Json` | jsonb |
| `Vec<T>`, `[T; N]` | `Array` | array casts from field type |
| Rust enums implementing `DbEnum` | `String` | Postgres enum casts |

UUID, timestamp, timestamptz, date, and enum inputs can be strings. With `with-uuid` and `with-chrono`, `uuid::Uuid`, `chrono::NaiveDate`, `chrono::NaiveDateTime`, and `chrono::DateTime<Tz>` can be passed directly. The renderer adds Postgres casts from the `FieldType`.

### Output

`fetch_all`, `fetch_one`, and `fetch_optional` return `tokio_postgres::Row`. `fetch_all_as::<T>` converts each row to JSON using selected `FieldType`s, then deserializes with serde.

| `FieldType` | JSON value | Struct field |
| --- | --- | --- |
| `Text`, `Citext`, `Uuid`, `Timestamp`, `Timestamptz`, `Date`, `Enum` | string | `String`, `uuid::Uuid`, chrono types with serde |
| `Integer` | number | `i32` |
| `BigInt` | number | `i64` |
| `Float` | number | `f64` |
| `Numeric` | string | `String` or an exact decimal wrapper |
| `Bool` | boolean | `bool` |
| `Jsonb` | JSON value | nested struct or `serde_json::Value` |
| `Bytea` | byte array | `Vec<u8>` |
| `Inet`, `Cidr`, `Range(elem)` | string | `String` or a serde-compatible newtype |
| `Custom(TypeSpec)` with `SelectRepr::Text` | string | `String` or a serde-compatible newtype |
| `Array(elem)` | JSON array | `Vec<T>` |

Root fields in joined queries use clean aliases like `id` and `email`. Joined flat fields still use aliases like `o_status`. Aggregate aliases are used exactly as given.

## Debug SQL

Use `build_pg()` or `build_rows_pg()` when you want to inspect SQL without executing it:

```rust
let built = select(orders())
    .fields([ID, STATUS])
    .filter(STATUS.eq("paid"))
    .build_pg()?;

println!("{}", built.rows.debug_sql());
println!("{}", built.count.debug_sql());
```

`debug_sql()` prints the SQL and the `Value` params. It is a development/debugging helper; execution still uses bind parameters.

## Connection And Pool

```rust
let db = rqb::connect("postgres://rqb:rqb@localhost:55432/rqb").await?;
let tls_db = rqb::connect_with_tls(database_url, tls).await?;

let custom = rqb::postgres::Db::from_pool(pool);
```

`connect` uses `NoTls` for local development. For cloud Postgres, pass a `tokio-postgres` TLS connector through `connect_with_tls` or `Db::connect_with_max_size_and_tls`.

`PgExecutor` is implemented for `tokio_postgres::Client`, `tokio_postgres::Transaction`, `deadpool_postgres::Client`, `Db`, and `Tx`. The same builder can run against any of them.

## Transactions

```rust
let tx = db.begin().await?;
insert(orders()).value(&new_order).execute(&tx).await?;
insert(order_items()).values(&items).execute(&tx).await?;
tx.commit().await?;
```

Dropping a transaction without `commit()` rolls it back. The closure-style `Db::transaction` API remains available, but begin/commit is the primary style.

Closure style:

```rust
db.transaction(txn!(|tx| {
    insert(orders()).value(&new_order).execute(tx).await?;
    insert(order_items()).values(&items).execute(tx).await?;
    Ok(())
}))
.await?;
```

Isolation level:

```rust
let tx = db.begin().serializable().await?;
insert(orders()).value(&new_order).execute(&tx).await?;
tx.commit().await?;
```

Savepoint:

```rust
let tx = db.begin().await?;
let sp = tx.savepoint("before_optional_step").await?;
let result = insert(orders()).value(&new_order).execute(&tx).await;
if result.is_err() {
    sp.rollback().await?;
}
tx.commit().await?;
```

Isolation levels: `.read_committed()`, `.repeatable_read()`, `.serializable()`. Transaction flags: `.read_only()`, `.deferrable()`.

## Error Handling

Runtime errors are structured:

```rust
match error {
    rqb::Error::NotFound => HttpStatus::NotFound,
    rqb::Error::UniqueViolation { .. } => HttpStatus::Conflict,
    rqb::Error::ForeignKeyViolation { .. } => HttpStatus::Conflict,
    e if e.is_retryable() => HttpStatus::ServiceUnavailable,
    e if e.constraint_name() == Some("app_users_email_key") => HttpStatus::Conflict,
    _ => HttpStatus::InternalServerError,
}
```

Use `fetch_optional` or `fetch_optional_as` when zero rows are a valid result. Use `is_retryable`, `constraint_name`, `table_name`, `column_name`, `code`, `detail`, and `hint` to map database errors into API errors. Direct enum matches remain the clearest choice for single variants such as `QueryCanceled`, `InsufficientPrivilege`, or `NotFound`.

## Code Generation

```bash
make db-up
cargo run -p rqb-cli -- generate \
  --database-url postgres://rqb:rqb@localhost:55432/rqb \
  --schema public \
  --out target/generated/rqb_schema.rs
```

The CLI introspects tables, views, arrays, JSONB columns, Postgres enums, and Postgres domains. Generated modules contain `Field` constants, `dataset()` functions, relation helpers, domain `TypeSpec` constants, and serde-compatible Rust enum wrappers. Generated enums serialize and deserialize as the exact Postgres labels, so fixed-shape DTOs can use them directly instead of validating status strings by hand.

Numeric domains are generated as custom field types that bind through exact decimal strings and select as text:

```rust
pub mod types {
    use rqb::prelude::*;

    pub const UINT_256: TypeSpec = TypeSpec::domain(Some("public"), "uint_256")
        .base(TypeFamily::Numeric)
        .value_repr(ValueRepr::DecimalString)
        .select_repr(SelectRepr::Text);
}

pub const AMOUNT: Field = Field::new("amount", FieldType::Custom(&types::UINT_256));
pub const AMOUNT_HISTORY: Field = Field::mapped(
    "amountHistory",
    "amount_history",
    FieldType::Array(ElemType::Custom(&types::UINT_256)),
)
.sortable(false);
```

That keeps values such as `uint_256` and large `numeric` amounts out of `f64`.
