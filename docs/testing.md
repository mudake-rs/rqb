# Testing Strategy

rqb keeps tests close to the layer that owns the behavior. The goal is fast feedback without hiding SQL behavior behind snapshots or a separate test crate.

## Layout

- `crates/rqb-core/src/validation/tests.rs`: validation behavior for fields, operators, values, grouping, joins, and write safety.
- `crates/rqb-postgres/src/tests.rs`: Postgres SQL rendering behavior with inline SQL assertions.
- `crates/rqb-postgres/tests/postgres_integration.rs`: runtime behavior against Postgres.
- `crates/rqb-cli/src/main.rs`: code generation unit tests.

Split a test file only when it becomes hard to navigate. Do not create a separate test crate while the workspace is still small.

## Naming

Use `{verb}_{what}_{condition}` so the test name reads like a specification:

```rust
#[test]
fn renders_left_join_with_qualified_columns() {}

#[test]
fn rejects_sort_on_unsortable_field() {}

#[tokio::test]
async fn executes_insert_update_delete_and_upsert() {}
```

Avoid numbered names such as `select_1`, generic names such as `it_works`, and names that only repeat the module name.

## Rendering Tests

Rendering tests should build a query and assert the produced SQL directly:

```rust
let built = select(orders_table().alias("o"))
    .left_join(users_table().alias("u"), field("o.userId").eq_col(field("u.id")))
    .fields([field("o.id"), field("u.email")])
    .build_rows_pg()
    .unwrap();

assert_eq!(
    built.sql,
    concat!(
        "SELECT \"o\".\"id\" AS \"o_id\", \"u\".\"email\" AS \"u_email\" ",
        "FROM \"orders\" AS \"o\" ",
        "LEFT JOIN \"app_users\" AS \"u\" ON \"o\".\"user_id\" = \"u\".\"id\" ",
        "LIMIT 100 OFFSET 0",
    )
);
assert_eq!(built.params.len(), 0);
```

Prefer inline `assert_eq!` plus `pretty_assertions` over snapshots for SQL. Snapshots are useful later for large generated schema output, not for compact SQL strings.

In test modules, import the pretty diffing assertion explicitly:

```rust
use pretty_assertions::assert_eq;
```

## Integration Tests

Use one command for the full Docker-backed test run:

```bash
make docker-test
```

That target starts a dedicated Compose project, runs Postgres on tmpfs, runs the Rust test suite in a Rust container, and always tears the project down on exit.

Integration tests use a shared Postgres schema from `tests/sql/init.sql` and wrap each client-backed test in `BEGIN`. `TestDb` rolls the transaction back on drop, so tests can run in parallel without leaving rows behind.

Expected database errors inside a transaction must use a savepoint:

```rust
client.batch_execute("SAVEPOINT duplicate").await?;
let error = insert(table).set(ID, existing_id).execute(&client).await.unwrap_err();
assert!(error.is_unique_violation());
client.batch_execute("ROLLBACK TO SAVEPOINT duplicate").await?;
```

Pool-specific tests may commit when the behavior under test is commit/rollback itself. Those tests must clean up committed rows explicitly.

## Feature Checklist

For a new operator:

- validation accepts the operator on supported fields
- validation rejects unsupported field types
- validation rejects invalid value shapes
- renderer emits expected SQL and parameters
- integration test covers real Postgres behavior when the operator is not trivial

For a new type:

- validation covers scalar and array value shapes
- renderer covers parameter casts and selection casts
- integration test round-trips insert/select behavior
- CLI test covers generated schema when introspection is involved

## Speed Budget

- Pure validation and rendering tests should stay under 5 seconds for `cargo test --workspace`.
- Docker-backed verification through `make docker-test` should stay under 30 seconds on a warm machine.
- Do not add per-test temporary databases or coverage tooling unless there is a clear current need.
