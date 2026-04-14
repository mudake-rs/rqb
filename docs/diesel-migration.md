# Migrating From Diesel

rqb is for dynamic Postgres queries in services. Diesel is for statically typed query construction and compile-time schema coupling. The tradeoff is deliberate: rqb gives simpler runtime composition; Diesel gives stronger compile-time guarantees.

For a longer application-level comparison with full examples, see [ergonomics.md](ergonomics.md).

| Diesel | rqb |
| --- | --- |
| `table.filter(status.eq("paid")).load::<Order>(&conn)` | `select(orders()).filter(STATUS.eq("paid")).fetch_all_as::<Order>(&db).await?` |
| table default selection / `all_columns` | omitted `.fields(...)` selects all selectable root dataset fields |
| `orders::table.find(id).first::<Order>(&conn)` | `select(orders()).filter(ID.eq(id)).fetch_one_as::<Order>(&db).await?` |
| `insert_into(orders).values(&new_order).get_result(&conn)` | `insert(orders()).value(&new_order).fetch_one_as::<Order>(&db).await?` |
| `insert_into(orders).values(&rows)` | `insert(orders()).values(&rows).execute(&db).await?` |
| `update(orders.find(id)).set(&patch)` | `update(orders()).set_from(&patch).filter(ID.eq(id))` |
| `delete(orders.filter(id.eq(id)))` | `delete(orders()).filter(ID.eq(id))` |
| `on_conflict(email).do_update().set(...)` | `insert(users()).value(&user).on_conflict(EMAIL).do_update([STATUS])` |
| `inner_join(users).select(...)` | `select(orders().alias("o")).join(users().alias("u"), USER_ID.on("o").eq_col(ID.on("u")))` |
| `filter(exists(...))` | `filter(exists(select(events().alias("e")).filter(EVENT_ORDER_ID.on("e").eq_col(ID.on("o")))))` |
| `filter(id.eq_any(subselect))` | `filter(ID.in_subquery(select(orders().alias("o")).fields([USER_ID.on("o")])))` |
| `.for_update().skip_locked()` | `.for_update().skip_locked()` |
| Postgres `DISTINCT ON` through Diesel DSL/extensions | `.distinct_on([USER_ID])` |
| Diesel transaction closure | `let tx = db.begin().await?; ...; tx.commit().await?` |
| Manual `sql_query` escape hatch | `raw("... ? ...").bind(value)` inside filters, CTEs, and assignments |

## Main Differences

No `schema.rs` or `table!` macro is required. You define `Field` constants by hand or generate them from Postgres with `rqb-cli`.

No mandatory `.fields(...)` for normal reads. Diesel lets `users::table.load::<User>(&mut conn)` use the table's default selection, usually all columns. rqb does the same kind of thing from `Dataset` metadata:

```rust
let users = select(users())
    .filter(STATUS.eq("active"))
    .fetch_all_as::<User>(&db)
    .await?;
```

On joins, rqb's omitted projection means root dataset fields only. Use `.fields([...])` to return joined columns.

No `Insertable`, `Queryable`, or `AsChangeset` derives are required. Writes use `Serialize`; reads use `Deserialize`.

No compile-time SQL verification. rqb validates against dataset metadata at runtime, then renders parameterized SQL.

Dynamic queries are first-class. `SearchRequest` can be deserialized from JSON and executed through the same validation path as hand-written builder code.

JSONB, JSON paths, arrays, Postgres enums, `json_agg`, raw CTEs, select CTEs, typed subqueries, row locks, `DISTINCT ON`, and typed bind rendering are built into the Postgres backend.

Transactions are runtime-oriented. `PgExecutor` works with `tokio_postgres::Client`, `tokio_postgres::Transaction`, pooled `Db`, and pooled `Tx`.

## Where rqb Is Different

Diesel's query shape is encoded in Rust types. That gives compile-time guarantees, but dynamic query composition often needs boxing and careful type work.

rqb keeps one runtime `SelectQuery` representation. A service can build a trusted SQL skeleton in Rust:

```rust
let query = select(users().alias("u"))
    .left_join(orders().alias("o"), ID.on("u").eq_col(USER_ID.on("o")))
    .filter(ORGANIZATION_ID.on("u").eq(current_org_id));
```

Then it can apply a JSON `SearchRequest`:

```rust
let page = query
    .request(search_request)
    .page_as::<serde_json::Value>(&db)
    .await?;
```

The JSON request is validated against dataset metadata. It can select fields, filter, sort, and page, but it cannot create joins, raw SQL, CTEs, or subqueries.

## What Diesel Still Does Better

Compile-time query verification.

Backend coverage beyond Postgres, including MySQL and SQLite.

Migrations.

Large ecosystem integrations around Diesel-specific derives and schema macros.
