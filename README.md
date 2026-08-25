# rqb 💩

Postgres-only Rust query builder on top of sqlx. It is not an ORM.

Home page: <https://mudake-rs.github.io/rqb/>

Start with [`samples`](samples) when you want code first. They are short,
compile-checked, and use the public API directly.

## Contents

- [Core Model](#core-model)
- [Status](#status)
- [Install](#install)
- [First Query](#first-query)
- [Execution](#execution)
- [Query Composition](#query-composition)
- [JSON Search](#json-search)
- [Raw SQL](#raw-sql)
- [Error Handling](#error-handling)
- [Writes](#writes)
- [Transactions](#transactions)
- [CLI](#cli)
- [Migrations And Schema Drift](#migrations-and-schema-drift)
- [Crates](#crates)
- [Checks](#checks)
- [License](#license)

## Core Model

- You write PostgreSQL, not generic SQL. rqb does not hide the dialect.
- Rust code owns query shape: sources, joins, projection, writes, raw fragments,
  locks, conflict handling, and returning clauses.
- Field metadata tells rqb which columns exist, which operators are allowed, and
  which fields may be exposed to JSON search.
- Values become sqlx Postgres bind arguments. User values are not interpolated
  into SQL.
- Client JSON can filter, sort, limit, and offset only through exposed metadata;
  it cannot define joins, raw SQL, CTEs, writes, or projections.

Validation runs before rendering. This is not a full compile-time SQL proof:
some shape and operator mistakes are rejected at `.build()?`, and PostgreSQL
remains the final authority for name resolution, constraints, permissions, and
query planning.

## Status

Pre-1.0. The repository is public and the supported distribution path is the
GitHub repository. APIs are still allowed to break when that makes the library
simpler or clearer.

The core builder targets Postgres 14+ for ordinary application queries. Some
helpers expose newer server features such as `MERGE` branches from Postgres 17
and UUIDv7 / old-new DML `RETURNING` from Postgres 18. The integration suite
runs against Postgres 18.

## Install

rqb is distributed through crates.io:

```toml
[dependencies]
rqb = "0.1.5"
chrono = "0.4.45"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
sqlx = { version = "0.9.0", default-features = false, features = [
    "postgres",
    "derive",
    "uuid",
    "chrono",
    "json",
    "runtime-tokio",
    "tls-rustls-ring-webpki",
] }
uuid = "1.23.4"
```

For source-pinned application builds, use the GitHub repository:

```toml
rqb = { git = "https://github.com/mudake-rs/rqb", rev = "<commit>" }
```

rqb does not enable a sqlx async runtime or TLS provider. Your application owns
that choice on its direct `sqlx` dependency; use any SQLx `runtime-*` and
`tls-*` combination that matches the rest of the process, such as
`tls-rustls-aws-lc-rs`, `tls-rustls-ring-webpki`, `tls-native-tls`, or
`tls-none`.

`uuid`, `chrono`, JSON, numeric, ranges, arrays, and other Postgres values are
accepted when the Rust type implements sqlx `Encode` and `Type` for Postgres.
Direct expression literals cover common bind types such as `Uuid`, chrono
date/time values, `PgInterval` / durations, `BigDecimal`, `Vec<u8>`, and
`serde_json::Value`; `param(value)` remains the explicit fallback for any other
sqlx-supported value.

## First Query

rqb needs schema metadata. For examples and small tests, define it inline with
`rqb::schema!`. For real applications, run `rqb generate`, check the generated
module into the app, and import it as `schema`; see [CLI](#cli).

Generated schema modules are normal Rust modules. `table()` / `view()` provide
the source metadata, uppercase constants are typed fields, and `alias("u")`
returns an alias-bound handle for joins and other multi-source queries.

```rust
use rqb::prelude::*;
use uuid::Uuid;

rqb::schema! {
    table public.app_users {
        id: uuid = Uuid,
        email: text = String,
        status: text = String,
    }
}

let query = select(app_users::table())
    .filter(app_users::STATUS.eq("active"))
    .order_asc(app_users::EMAIL)
    .limit(20)
    .build()?;
```

Examples below use `schema::users::*` as a placeholder for whatever module your
generated schema lives in. Samples in `samples/` use imports such as
`use rqb_sample_schema::app_users as users;`.

`select(table())` does not render `SELECT *`. It renders the known root fields
from metadata, so SQL output stays explicit and stable.

Use `.columns((...))` when a query intentionally returns a narrower response
shape:

```rust
let query = select(app_users::table())
    .columns((app_users::ID, app_users::EMAIL))
    .filter(app_users::STATUS.eq("active"))
    .build()?;
```

`query.sql` contains `$N` placeholders and `query.arguments()?` creates
`sqlx::postgres::PgArguments` at execution time. Use `query.pretty_sql()` for
formatted SQL only, or `query.summary()` for debug logs that show
pretty-printed SQL, bind count, short bind type names for common types, and
cacheability without interpolating values into SQL.

## Execution

Use any sqlx executor:

```rust
#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
}

let rows = select(schema::users::table())
    .filter(schema::users::STATUS.eq("active"))
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

Built queries pass `cacheable` to sqlx `.persistent(...)`. For Postgres, sqlx
keeps a bounded prepared-statement cache per connection, defaulting to 100
entries with LRU eviction; tune it with
`PgConnectOptions::statement_cache_capacity(...)` or the
`statement-cache-capacity` URL parameter. rqb marks raw SQL fragments as
non-cacheable because it cannot prove their text is a stable statement shape.
Typed queries remain cacheable, but high-cardinality generated shapes such as
many different `IN` list lengths or optional-filter combinations can churn the
cache even though memory stays bounded.

Service functions can usually accept `&PgPool`. When a query must be reused
inside a transaction, prefer reusing the query shape instead of making the async
service function generic over an executor:

```rust
fn find_user_query(id: Uuid) -> Select {
    select(schema::users::table())
        .filter(schema::users::ID.eq(id))
}

async fn find_user(pool: &PgPool, id: Uuid) -> rqb::Result<UserRow> {
    find_user_query(id).fetch_one_as::<UserRow>(pool).await
}

tx!(&pool, |conn| {
    let user = find_user_query(id)
        .fetch_one_as::<UserRow>(&mut *conn)
        .await?;
    Ok(user)
})
.await?;
```

Use `impl PgExecutor<'_>` for small helpers that should execute directly from
both pool-backed code and transaction code:

```rust
async fn cancel_open_orders(db: impl PgExecutor<'_>, user_id: Uuid) -> rqb::Result<u64> {
    update(schema::orders::table())
        .set(schema::orders::STATUS.set("canceled"))
        .filter(schema::orders::USER_ID.eq(user_id))
        .filter(schema::orders::STATUS.eq("open"))
        .execute(db)
        .await
}
```

When passing a sqlx transaction or pool connection to an executor helper, use
sqlx's reborrow pattern: `&mut *tx` or `&mut *conn`.

Statement convenience methods such as `fetch_all_as::<T>(&pool)` build,
validate, and render the SQL for that call. If a hot path executes the same
server-owned query shape repeatedly, build once and reuse the `BuiltQuery`:

```rust
let query = select(schema::users::table())
    .columns((schema::users::ID, schema::users::EMAIL))
    .filter(schema::users::STATUS.eq("active"))
    .build()?;

let rows = query.fetch_all_as::<UserRow>(&pool).await?;
```

Scalar queries use `fetch_scalar::<T>()` for many rows or
`fetch_one_scalar::<T>()` for one row. Raw SQL uses `raw("... ? ...")` with `?`
placeholders. `??` renders a literal question mark.

For HTTP response streams, pass a cloned pool handle into the owned streaming
helpers: `fetch_stream_pool`, `fetch_stream_pool_as::<T>()`, or
`fetch_stream_pool_scalar::<T>()`. The returned stream owns the built query and
pool handle. `BuiltQuery::fetch_stream*` remains available when you build once
and keep the built query alive yourself. HTTP handlers should still choose their
own chunking policy; rqb yields rows, application code formats response chunks.

## Query Composition

Rust code owns joins, CTEs, subqueries, set queries, aggregates, windows, locks,
and write conflict handling. Client JSON never defines these shapes.

In this section: [Builder Surface](#builder-surface),
[Selected Columns](#selected-columns), [Counts](#counts),
[Postgres Semantics](#postgres-semantics), [Derived Sources](#derived-sources),
[DSL Helpers](#dsl-helpers), [Window Expressions](#window-expressions), and
[SQL Vocabulary Values](#sql-vocabulary-values).

Use alias handles as soon as a query or write has more than one source. rqb can
validate field capabilities and query shape, but PostgreSQL owns final name
resolution; unqualified columns that exist in both sources can still fail at
execution with an ambiguous-column error. The same applies to `MERGE`: alias the
target and incoming source when the `ON` clause or branch conditions compare
same-named columns.

```rust
use rqb::dsl::{exists, sum};
use rqb::prelude::*;

let paid_orders = select(schema::orders::table())
    .columns((schema::orders::USER_ID, schema::orders::TOTAL_CENTS))
    .filter(schema::orders::STATUS.eq("paid"))
    .infer_cte("paid_orders")?;

let po = paid_orders.source().alias("po");
let u = schema::users::alias("u");

let rows = select(&u)
    .with(paid_orders)
    .join(po, u.id().eq_field(schema::orders::USER_ID.at("po")))
    .filter(exists(
        select(schema::orders::table())
            .column(schema::orders::ID)
            .filter(schema::orders::USER_ID.eq_field(u.id())),
    ))
    .expr_as(sum(schema::orders::TOTAL_CENTS.at("po")), "paid_total")
    .group_by(u.id())
    .fetch_all_as::<UserRow>(&pool)
    .await?;
```

### Builder Surface

rqb covers common Postgres application-query shapes without trying to mirror the
whole SQL grammar:

- Projection: `column`, `columns`, `expr`, `expr_as`, and `default_columns`.
- Filtering and paging: `filter_if`, `filter_option`, `or_filter_option`,
  row-value cursor predicates, `limit`, `offset`, and `Select::count`.
- Derived sources: joins, CTEs, subqueries, set queries, `values_source`, and
  set-returning function sources such as `generate_series_source`.
- Aggregation: `group_by`, `having`, `count_distinct`, aggregate `FILTER`, and
  window expressions.
- Postgres data shapes: array, JSONB, range, full-text, date/time, UUID, and
  scalar expression helpers in `rqb::dsl`.
- Writes: `set_many`, `set_if`, `set_option`, `insert(...).from_select(...)`,
  upserts, batch `VALUES`, and `merge_into`.
- Locks: row-lock builders on `SELECT` and transaction-scoped advisory-lock
  helpers in `rqb::dsl`.

### Selected Columns

Projection calls are explicit: `.column(...)`, `.columns(...)`, `.expr(...)`,
and `.expr_as(...)` replace the default root projection with the values you
name.
When a query should return the normal root fields plus computed columns, expand
the root fields first:

```rust
use rqb::dsl::length;

let query = select(schema::orders::table())
    .default_columns()
    .expr_as(length(schema::orders::STATUS), "status_length")
    .build()?;
```

### Counts

`Select::count()` is a matching-row count helper, not a locked-query replay. It
removes ordering, page limits, `FETCH`, and row-lock clauses before wrapping the
query in `count(*)`. For `FOR UPDATE SKIP LOCKED`-style workflows, count the
locked result set explicitly in server-owned SQL if lock semantics matter.
REST-style pagination stays in application code; the REST sample shows
`limit` / `offset`, cursor pagination, and `Select::count()` for a matching
count query.

### Postgres Semantics

Postgres `MERGE ... RETURNING` reports rows affected by each executed action,
including rows deleted by `WHEN NOT MATCHED BY SOURCE THEN DELETE`. If an API
needs the final table state after that branch, run a follow-up `SELECT` with the
same server-owned scope.

MERGE actions are validated against Postgres `WHEN` clause rules before
rendering.

Grouped analytics can make non-null source columns nullable in result rows.
`ROLLUP`, `CUBE`, and `GROUPING SETS` emit subtotal rows by replacing grouped
dimension values with `NULL`, so downstream `sqlx::FromRow` structs should use
`Option<T>` for those projected dimensions.

### Derived Sources

For derived sources, rqb needs exposed field metadata. `Select::infer_cte`
and `Select::infer_source` infer it from explicit field projections.
Computed columns can use `rqb::field!`:

```rust
use rqb::dsl::{case, count_all};

let item_count = rqb::field!("item_count": int8 => i64, ordered);

let order_size = case()
    .when(schema::orders::TOTAL_CENTS.gte(10_000), "large")
    .else_("standard");
```

Raw SQL, computed columns with custom aliases, and renamed projections can still
use the explicit `cte(...)`, `subquery(...)`, or `raw_source(...)` constructors.

### DSL Helpers

SQL expression helpers are available as qualified `rqb::...` calls, but they
live outside the prelude so broad names like `left`, `right`, `lower`,
`replace`, `row`, and `array` do not pollute every service module. Use
`rqb::dsl::*` for short query modules, or import the exact helpers a module
needs:

```rust
use rqb::dsl::{coalesce, count_all, date_trunc_part, sum, window, DatePart};
use rqb::prelude::*;
```

Common helper families in the flat catalog:

| Family | Examples |
| --- | --- |
| Boolean predicates | `and`, `or`, `not`, `exists`, `true_`, `false_` |
| Aggregates | `count_all`, `sum`, `sum_distinct`, `avg_distinct`, `jsonb_agg_object`, `percentile_cont` |
| Arrays | `array`, `array_length`, `array_position`, `trim_array`, `unnest` |
| Date and time | `now`, `date_trunc`, `date_trunc_part`, `date_bin`, `to_char`, `isfinite` |
| Full-text search | `to_tsvector`, `phraseto_tsquery`, `ts_rank`, `ts_headline` |
| JSON/JSONB | `json_build_object`, `jsonb_build_object`, `jsonb_pretty`, `array_to_json` |
| Math | `round`, `sqrt`, `pow`, `random_between`, `width_bucket` |
| Range | `range_lower`, `range_upper`, `range_merge`, `multirange_merge`, `isempty`, `lower_inc` |
| Scalar expressions | `case`, `coalesce`, `null`, `literal`, `greatest`, `current_user`, `scalar_subquery` |
| Set-returning sources | `generate_series_source`, `unnest_source`, `json_each_source`, `regexp_split_to_table_source`, `values_source` |
| Text | `lower`, `format`, `translate`, `repeat`, `octet_length`, `encode` |
| UUID | `uuidv7`, `uuid_extract_timestamp`, `gen_random_uuid` |
| Window functions | `window`, `row_number`, `rank`, `lag`, `preceding`, `unbounded_preceding` |

### Window Expressions

Aggregate calls can also be used as PostgreSQL window functions:

```rust
select(schema::orders::table())
    .column(schema::orders::USER_ID)
    .expr_as(
        sum(schema::orders::TOTAL_CENTS).over(window().partition_by(schema::orders::USER_ID)),
        "user_total_cents",
    );
```

### SQL Vocabulary Values

When PostgreSQL expects a stable SQL vocabulary literal rather than user data,
use typed helpers or `literal(...)` instead of a bind parameter:

```rust
use rqb::dsl::{DatePart, date_trunc_part, literal};

let day = date_trunc_part(DatePart::Day, schema::orders::CREATED_AT);
let month = rqb::date_trunc(literal("month"), schema::orders::CREATED_AT);
```

## JSON Search

`SearchRequest` is for client-controlled search parameters only. It cannot
define tables, joins, raw SQL, subqueries, writes, computed projections, or
response fields.

```json
{
  "filter": {
    "and": [
      { "field": "status", "operator": "equals", "value": "paid" },
      { "field": "total_cents", "operator": "gte", "value": 5000 }
    ]
  },
  "sort": [{ "field": "created_at", "dir": "desc" }],
  "limit": 20,
  "offset": 0
}
```

Apply it to a trusted Rust query:

```rust
let query = select(schema::order_search_view::view())
    .filter(schema::order_search_view::ORGANIZATION_ID.eq(current_org_id))
    .apply_search(search_request)?
    .build()?;
```

Server filters are preserved and combined with the request filter using `AND`.
Request sort, limit, and offset are request-owned clauses and replace existing
builder values. Only fields with `Meta::json(...)` are visible to JSON requests.
Operators are gated by field capabilities: equality/null tests require equality
capability, sort requires ordering capability, and LIKE/regex/text-pattern
operators require text-pattern capability such as `OpSet::text()`.
Client-supplied LIKE and regex patterns are capped at 1024 Unicode scalar
values; public APIs should still set a database `statement_timeout` appropriate
for their workload.

> Tenant and permission scope: install tenant, user, RBAC, and soft-delete
> filters before calling `apply_search`. If a request is allowed to own the
> entire search clause, start from a fresh `select(...)` and apply it there.
> Cursor endpoints should not accept a full `SearchRequest`; accept a
> filter-only DTO with `#[serde(deny_unknown_fields)]` so clients cannot supply
> cursor-breaking `sort`, `limit`, or `offset` fields.

Search failures are ordinary `rqb::Error` variants, so API boundaries can return
stable client errors without parsing strings:

| Error | Usual HTTP shape |
| --- | --- |
| `InvalidSearchField` | `400`, unknown field |
| `SearchFieldNotExposed` | `400`, field exists but is not exposed to JSON search |
| `InvalidSearchOperator` | `400`, operator is not allowed for that field |
| `InvalidSearchValue` | `400`, JSON value has the wrong shape for the field kind |
| `InvalidSort` | `400`, field is visible but not sortable |
| `EmptySearchLogical` | `400`, `and` / `or` group is empty |

See `samples/json-search` for exact payloads and error mapping. Malformed JSON
body errors happen before rqb sees a `SearchRequest`; in axum, map the `Json`
extractor rejection at the HTTP boundary when parse failures should use the
same client error envelope as rqb validation failures.

## Raw SQL

The typed DSL is not meant to mirror every PostgreSQL catalog function. For
simple unlisted functions, use `function(...)`, `aggregate(...)`, or
`function_source(...)`. For PostgreSQL syntax that does not fit those shapes,
rqb exposes raw constructors at the exact AST slot:

| Helper | Returns | Use for |
| --- | --- | --- |
| `raw(...)` | `RawStmt` | A full server-owned statement |
| `raw_expr(...)` | `ValueExpr` | A projection, assignment value, or comparison side |
| `raw_predicate(...)` | `BoolExpr` | A `WHERE`, `ON`, or `HAVING` predicate |
| `raw_source(...)` | `Source` | A derived table or set-returning expression in `FROM` |

Raw fragments use rqb `?` placeholders outside SQL quoted contexts. Question
marks inside single-quoted strings, dollar-quoted bodies, quoted identifiers,
and comments stay literal. Escape literal question marks in SQL operator
positions as `??`, for example Postgres JSONB `?` operators. Raw fragments are
validated for bind-count mismatches and numbered together with the surrounding
typed query:

```rust
use rqb::prelude::*;

let extension_score = raw_expr(
    "custom_extension_score(payload, ?::text)",
    [Param::typed("strict".to_owned())],
);

let extension_rows = raw_source(
    "SELECT * FROM custom_extension_scan(?::text)",
    "ext",
    vec![
        Param::typed("tenant-42".to_owned()),
    ],
    rqb::field!("id": uuid => uuid::Uuid, equality),
);

let scored = select(extension_rows)
    .expr_as(extension_score, "extension_score");
```

`raw("...")` has fluent `.bind(value)` calls. Slot-level raw fragments
currently take already-erased params, so use `Param::typed(value)` for
`raw_expr(...)`, `raw_predicate(...)`, and `raw_source(...)`, especially when a
single fragment binds values of different Rust types.

Any raw fragment marks the whole built query as non-cacheable, so execution uses
`sqlx` with persistent prepared-statement caching disabled for that query. Keep
raw fragments for server-owned SQL that genuinely needs them; prefer typed DSL
helpers when they express the same stable SQL shape.

Generated schemas keep unknown extension columns as raw-only metadata constants
such as `EMBEDDING_META`. Use `*_META.expr()` / `*_META.at("alias")` with
`op(...)`, `cast(...)`, or raw helpers when a column is deliberately outside
rqb's typed `Field<T>` catalog.

## Error Handling

rqb returns one structured `Error` enum for validation failures, sqlx execution
failures, and mapped Postgres SQLSTATE errors. Application code should match on
variants, not parse database message strings.

The usual HTTP mapping is:

- `NotFound` -> `404 Not Found`.
- `UniqueViolation` / `ExclusionViolation` -> `409 Conflict`.
  Use `constraint_name()` when logging or building domain-specific messages.
- `ForeignKeyViolation`, `RestrictViolation`, `NotNullViolation`, and
  `CheckViolation` -> `400 Bad Request`.
- `InvalidSearchField`, `SearchFieldNotExposed`, `InvalidSearchOperator`,
  `InvalidSearchValue`, `EmptySearchLogical`, and `InvalidSort` ->
  `400 Bad Request`.
- `SerializationFailure`, `DeadlockDetected`, and connection failures ->
  `503 Service Unavailable` or a retry response. `error.is_retryable()` returns
  true for these retryable cases.
- `LockNotAvailable` (`55P03`, for example `FOR UPDATE NOWAIT`) -> `409 Conflict`,
  `423 Locked`, or a domain-specific "already busy" response.
- `QueryCanceled` -> `504 Gateway Timeout` or request timeout, depending on
  who canceled the query.
- `InsufficientPrivilege` -> `403 Forbidden`.
- Builder-shape errors such as `DeleteWithoutFilter`, `RawBindMismatch`,
  `InvalidInsertShape`, `InvalidCteShape`, and `InvalidRowShape` are usually
  `500 Internal Server Error` unless they came directly from a JSON request.

See `samples/error-handling` for standalone matching examples and
`samples/rest-api/src/error.rs` for the same pattern inside an axum
`IntoResponse` boundary.

## Writes

Writes use field assignments or derive-generated assignments. There is no
serde write bridge.

In this section: [Assignments](#assignments),
[Database Defaults](#database-defaults), [Conditional Changes](#conditional-changes),
[Writes With CTEs](#writes-with-ctes), [DTO Mappings](#dto-mappings),
[Upserts And Batch Inserts](#upserts-and-batch-inserts), and
[Concurrency](#concurrency).

### Assignments

```rust
let created = insert(schema::users::table())
    .set_many((
        schema::users::ID.set(user_id),
        schema::users::EMAIL.set("ada@example.com"),
        schema::users::STATUS.set("active"),
    ))
    .returning(schema::users::ID)
    .fetch_one_scalar::<Uuid>(&pool)
    .await?;

update(schema::users::table())
    .set(schema::users::STATUS.set("disabled"))
    .filter(schema::users::ID.eq(user_id))
    .execute(&pool)
    .await?;
```

`returning(...)` accepts either one field or a tuple/list of fields:

```rust
let query = insert(schema::users::table())
    .set_many((
        schema::users::ID.set(user_id),
        schema::users::EMAIL.set("ada@example.com"),
    ))
    .returning((schema::users::ID, schema::users::EMAIL))
    .build()?;
```

Computed writes use `set_expr(...)`, and PostgreSQL 18 `RETURNING old.field` /
`new.field` is available on generated fields:

```rust
let changed = update(schema::users::table())
    .set(schema::users::LOGIN_COUNT.set_expr(
        schema::users::LOGIN_COUNT.expr().op("+", 1),
    ))
    .filter(schema::users::ID.eq(user_id))
    .returning_as(schema::users::LOGIN_COUNT.old_value(), "old_login_count")
    .returning_as(schema::users::LOGIN_COUNT.new_value(), "new_login_count")
    .fetch_one_as::<LoginCountChange>(&pool)
    .await?;
```

Use `set_null()` when an application intentionally writes SQL `NULL`, and
`set_default()` when PostgreSQL should apply the column default:

```rust
update(schema::invoices::table())
    .set(schema::invoices::PAID_AT.set_null())
    .filter(schema::invoices::ID.eq(invoice_id))
    .execute(&pool)
    .await?;

update(schema::invoices::table())
    .set(schema::invoices::STATE.set_default())
    .filter(schema::invoices::ID.eq(invoice_id))
    .execute(&pool)
    .await?;
```

rqb renders the assignment; PostgreSQL still enforces `NOT NULL` constraints at
execution time.

### Database Defaults

For a row populated entirely by database defaults, use `DEFAULT VALUES`:

```rust
let id = insert(schema::jobs::table())
    .default_values()
    .returning(schema::jobs::ID)
    .fetch_one_scalar::<Uuid>(&pool)
    .await?;
```

### Conditional Changes

Conditional write helpers keep service code linear when a field depends on
application state:

```rust
update(schema::users::table())
    .set_option(new_email, |email| schema::users::EMAIL.set(email))
    .set_option(new_status, |status| schema::users::STATUS.set(status))
    .filter(schema::users::ID.eq(user_id))
    .filter_option(current_status, |status| schema::users::STATUS.eq(status))
    .execute(&pool)
    .await?;
```

### Writes With CTEs

`INSERT`, `UPDATE`, and `DELETE` also accept CTEs, so write statements can stay
in the typed builder instead of falling back to raw SQL:

```rust
let active_ids = select(schema::users::table())
    .column(schema::users::ID)
    .filter(schema::users::ACTIVE.eq(true))
    .infer_cte("active_ids")?;
let active_ids_source = active_ids.source().alias("ids");
let u = schema::users::alias("u");

update(&u)
    .with(active_ids)
    .set(schema::users::STATUS.set("active"))
    .from(active_ids_source)
    .filter(u.id().eq_field(schema::users::ID.at("ids")))
    .execute(&pool)
    .await?;
```

Derived sources such as CTEs, subqueries, and raw sources expose metadata but
do not yet have generated alias-bound handles. For repeated references, keep
the alias in one constant and use `.at(ALIAS)`:

```rust
const IDS: &str = "ids";

let active_ids_source = active_ids.source().alias(IDS);

update(&u)
    .with(active_ids)
    .from(active_ids_source)
    .filter(u.id().eq_field(schema::users::ID.at(IDS)));
```

### DTO Mappings

With generated schema modules, request DTOs can derive write mappings:

```rust
#[derive(rqb::Insertable)]
#[rqb(table = schema::users)]
struct NewUser {
    email: String,
    status: String,
}

let created = insert(schema::users::table())
    .set(schema::users::ID.set(user_id))
    .values(&new_user)
    .returning(schema::users::ID)
    .fetch_one_scalar::<Uuid>(&pool)
    .await?;
```

The derive maps Rust fields to generated schema fields. It does not serialize
the whole DTO through `serde_json`, so database types stay on the sqlx encode
path.

Result DTOs can also decode JSON aggregate columns into typed nested structs.
rqb builds the JSON shape; sqlx and serde decode the returned `jsonb` column:

```rust
#[derive(serde::Deserialize, serde::Serialize)]
struct OrderSummaryItem {
    id: Uuid,
    status: String,
    total_cents: i64,
}

#[derive(serde::Serialize, sqlx::FromRow)]
struct UserOrderSummaryRow {
    email: String,
    order_count: i64,
    #[sqlx(json)]
    orders: Vec<OrderSummaryItem>,
}

let orders_json = coalesce([
    jsonb_agg_object![o.id(), o.status(), o.total_cents()]
        .aggregate_order_desc(o.created_at())
        .aggregate_filter(o.id().is_not_null()),
    literal("[]").cast("jsonb"),
]);
```

`jsonb_agg_object!` uses field metadata keys such as `total_cents`, so the
nested DTO field names, or their serde renames, must match. The `coalesce`
keeps empty left-join groups as `[]` instead of SQL `NULL`.

Derives map `snake_case` Rust fields to generated `SHOUTY_SNAKE_CASE` schema
constants by default. Use `#[rqb(field = schema::table::FIELD)]` when a DTO field
name differs from the database column, and `#[rqb(skip)]` for local-only fields.
For `Insertable`, `#[rqb(skip_none)]` skips `None` on an `Option<T>` field;
otherwise the `Option<T>` itself is inserted as the value. `Changeset` always
treats `Option<T>` as patch semantics: `Some` sets the column, `None` leaves it
unchanged.

### Upserts And Batch Inserts

Upserts can update several columns from `EXCLUDED` without repeating
`set_excluded()` per field:

```rust
insert(schema::products::table())
    .values(&product)
    .on_conflict(schema::products::SKU)
    .do_update_excluded((
        schema::products::NAME,
        schema::products::PRICE_CENTS,
        schema::products::ATTRIBUTES,
        schema::products::TAGS,
    ))
    .returning_all()
    .fetch_one_as::<ProductRow>(&pool)
    .await?;
```

For sync-style batch DTO inputs, `values_many(...)` exposes the incoming
rows as an inline `VALUES` source. `set_from("alias")` copies fields from that
same source in conflict updates:

```rust
let incoming = [product_a, product_b];

insert(schema::products::table())
    .values_many(&incoming, "incoming")?
    .on_conflict(schema::products::SKU)
    .do_update_set((
        schema::products::NAME.set_from("incoming"),
        schema::products::PRICE_CENTS.set_from("incoming"),
    ))
    .execute(&pool)
    .await?;
```

### Concurrency

Optimistic locking stays normal update SQL: include both the row id and expected
version in `WHERE`, increment the version in `SET`, and use `RETURNING` to tell
success from a version miss.

```rust
let updated = update(schema::orders::table())
    .set_many((
        schema::orders::STATUS.set("paid"),
        schema::orders::VERSION.set_expr(schema::orders::VERSION.expr().op("+", 1)),
    ))
    .filter(schema::orders::ID.eq(order_id))
    .filter(schema::orders::VERSION.eq(expected_version))
    .returning_all()
    .fetch_optional_as::<OrderRow>(&pool)
    .await?;
```

Row locks stay on `SELECT` and should normally run inside a transaction:

```rust
let order = select(schema::orders::table())
    .filter(schema::orders::ID.eq(order_id))
    .for_update()
    .nowait()
    .fetch_one_as::<OrderRow>(&mut *tx)
    .await?;
```

Use `for_no_key_update`, `for_share`, or `for_key_share` when the narrower
Postgres row-lock mode matches the workflow. `skip_locked()` supports worker
queue patterns where busy rows should be ignored.

Postgres advisory locks are exposed as transaction-scoped statement helpers in
`rqb::dsl`:

```rust
use rqb::dsl::try_advisory_xact_lock_named;

let acquired = try_advisory_xact_lock_named(format!("job:{job_id}"))
    .fetch_one_scalar::<bool>(&mut *tx)
    .await?;
if !acquired {
    return Ok(None);
}
```

Postgres advisory locks accept `bigint` or `(int4, int4)` keys. The `_named`
helpers hash a string with a stable FNV-1a 64-bit hash and bind the resulting
`bigint`; collisions are possible, so use explicit numeric keys for
collision-critical protocols. Session-level advisory locks remain available
through `raw(...)`, but transaction-level locks are safer with connection pools.

`DELETE` without a filter is rejected during validation.

## Transactions

rqb executes through sqlx. Use `tx!` when several statements must commit or
roll back together:

```rust
tx!(&pool, |conn| {
    let created_id = insert(schema::users::table())
        .set_many((
            schema::users::ID.set(user_id),
            schema::users::EMAIL.set("ada@example.com"),
        ))
        .returning(schema::users::ID)
        .fetch_one_scalar::<Uuid>(conn)
        .await?;
    Ok(created_id)
})
.await?;
```

Use explicit `pool.begin().await?` / `commit().await?` when you need sqlx
features that do not fit closure scope, such as savepoints, custom isolation
setup, or transaction ownership that crosses helper boundaries.

## CLI

`rqb-cli` introspects Postgres and writes a compact `rqb::schema!` module. The
macro expands to `Meta`, `Field<T>`, `FIELDS`, and `table()` / `view()` items.

Install the CLI from crates.io; the installed binary is `rqb`.

```bash
cargo install rqb-cli
```

The package name is `rqb-cli`; the binary name is `rqb`.

Inside this workspace, run the same binary through `cargo run -p rqb-cli --`.

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --out src/schema.rs
```

Known sqlx-supported Postgres types generate typed `Field<T>` constants.
Postgres enum types generate Rust enums with `sqlx::Type`, and enum columns
use those generated types in `Field<T>`. Schema crates that contain generated
enums need a direct `sqlx` dependency with the `derive` and `postgres` features.
Enum typing is scoped to the generated Postgres schema; enum types from other
schemas safely fall back to raw-only metadata.

Unknown domains and extension types stay raw-only metadata: they can be part of
server-owned SQL shape, but they are hidden from JSON requests by default.
Server-owned extension operators can be built from raw-only metadata:

```rust
let embedding = vector_documents::EMBEDDING_META.expr();
let distance = embedding.op("<->", rqb::dsl::param("[0.1,0.2,0.3]".to_owned()).cast("vector"));
```

Project-owned types can be mapped with a checked-in TOML config:

```toml
[type_map."bitcoin.uint256"]
rust = "crate::types::PgU256"
ops = "ordered"
json = "text"
array = true

[raw_only]
allow = ["public.vector_documents.embedding"]
```

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --config rqb.toml \
  --out src/schema.rs
```

The generated field uses the Rust path directly. The mapped type must implement
the relevant sqlx Postgres traits; rqb does not convert custom values itself.
Custom mappings are not exposed to JSON search unless `json` is set explicitly.
Array columns remain hidden from JSON search even when their scalar type mapping
sets `json`; rqb does not currently define client JSON semantics for array
filters.

Use `--report` to see generated schema counts, raw-only columns, and unused
`type_map` entries. CI can fail on unexpected raw-only columns or stale custom
type mappings:

```bash
rqb generate \
  --database-url "$DATABASE_URL" \
  --schema public \
  --config rqb.toml \
  --out src/schema.rs \
  --check \
  --report \
  --deny-raw-only \
  --deny-unused-type-map
```

Generated primary-key and unique constraints are exposed inside each relation's
`constraints` module for conflict handling:

```rust
insert(schema::users::table())
    .on_conflict_constraint(schema::users::constraints::USERS_EMAIL_KEY)
    .do_nothing();
```

Generated field names match database column names. HTTP JSON casing belongs in
application DTOs, not in generated schema metadata.

The CLI also annotates nullable, generated, identity, and materialized-view
metadata in comments. It does not generate row structs; use `Option<T>` in
`sqlx::FromRow` structs for nullable columns.

## Migrations And Schema Drift

rqb does not run or generate database migrations. Keep schema changes in SQL
migration files and apply them with `sqlx::migrate!`, `sqlx migrate`, refinery,
sqitch, psql, or your deployment system.

The intended flow is:

1. Apply migrations to a real Postgres database.
2. Regenerate the schema module:

   ```bash
   rqb generate \
     --database-url "$DATABASE_URL" \
     --schema public \
     --out src/schema.rs
   ```

3. Commit the generated schema module with the migration.
4. In CI, run the same command with `--check` to catch drift between the
   database schema and the checked-in generated module.

`rqb-cli` introspects the schema that already exists. It does not diff schema
versions, generate `ALTER TABLE`, or decide migration ordering. When a column is
renamed, removed, or changes type, regenerate the schema module and let Rust
compile errors point at stale query code.

## Crates

- `rqb`: typed AST, renderer, params, execution helpers, and public API.
- `rqb-macros`: procedural macros re-exported by `rqb`.
- `rqb-cli`: schema introspection and code generation.

## Checks

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
make verify
```

## License

Licensed under either Apache-2.0 or MIT, at your option.
