# rqb sample REST API

Small actix-web app that uses rqb for reads, writes, joins, JSONB, batch insert, transactions, typed UUID/date-time rows, generated enum DTOs, validation, shared generated schema metadata, and JSON `SearchRequest`.

The code is split into thin HTTP handlers, request DTOs, input validation, pagination responses, and a `src/db/` layer. The DB layer owns write/read models and services where rqb is used directly; generated metadata comes from `samples/sample-base`.

The order service intentionally shows both transaction styles: `create` uses explicit `begin()/commit()`, while `delete` uses closure-style `transaction(txn!(...))`.

The schema in `samples/sample-base/src/schema.rs` is produced by `rqb-cli` from the Postgres schema. After changing `tests/sql/init.sql`, regenerate it from the repository root:

```bash
make generate-sample-base-schema
```

## Run

From the repository root:

```bash
make docker-infra-up
cargo run --manifest-path samples/rest-api/Cargo.toml
```

The API listens on `127.0.0.1:3000` and connects to `postgres://rqb:rqb@localhost:55432/rqb` unless `DATABASE_URL` is set.

The sample is intentionally not a workspace member, so use `--manifest-path`.

## What To Read

```text
../sample-base/src/schema.rs generated rqb metadata and relation helpers
src/db/orders.rs       DB models plus OrderService queries/writes
src/db/users.rs        joined user/order aggregate query
src/orders/handlers.rs thin HTTP layer, validation, transaction boundaries
src/orders/requests.rs HTTP DTOs only
```

The DB layer does not depend on HTTP DTOs. Handlers convert request DTOs into DB write/query models; fixed-shape DB models are returned directly when the API shape is the same.

## Endpoints

```text
GET    /orders
POST   /orders
GET    /orders/{id}
PATCH  /orders/{id}
DELETE /orders/{id}
GET    /orders/stats
POST   /orders/search
GET    /users
POST   /users
GET    /users/{id}
PATCH  /users/{id}
```

## Curl

```bash
curl 'http://127.0.0.1:3000/orders?status=paid&limit=5'
curl 'http://127.0.0.1:3000/orders/stats'
curl 'http://127.0.0.1:3000/users/10000000-0000-0000-0000-000000000001'
```

JSON search body:

```bash
curl -X POST 'http://127.0.0.1:3000/orders/search' \
  -H 'Content-Type: application/json' \
  -d '{
    "fields": ["id", "email", "status", "totalCents"],
    "limit": 5,
    "sort": [{ "field": "createdAt", "dir": "desc" }],
    "filter": {
      "and": [
        { "field": "status", "operator": "equals", "value": "paid" },
        { "field": "metadata.score", "operator": "gte", "value": 80 }
      ]
    }
  }'
```

This endpoint applies the JSON `SearchRequest` to `order_search_view`, a server-owned search view. The client can choose fields, filters, sort, limit, and offset; it cannot define joins or raw SQL.

Create order:

```bash
curl -X POST 'http://127.0.0.1:3000/orders' \
  -H 'Content-Type: application/json' \
  -d '{
    "userId": "10000000-0000-0000-0000-000000000001",
    "channel": "web",
    "status": "draft",
    "metadata": { "score": 72, "campaign": "sample" },
    "tags": ["sample"],
    "items": [
      {
        "productId": "20000000-0000-0000-0000-000000000001",
        "quantity": 1,
        "unitPriceCents": 10900,
        "metadata": { "warehouse": "ams" }
      }
    ]
  }'
```
