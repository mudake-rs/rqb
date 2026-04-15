# rqb samples

These samples are small, standalone pressure tests for the public rqb API.
All standalone samples use generated schema metadata from `samples/sample-base`
instead of redefining table fields by hand.

Regenerate the shared sample schema after changing `tests/sql/init.sql`:

```bash
make generate-sample-base-schema
```

Start the shared Postgres fixture from the repository root:

```bash
make db-up
```

Then run samples directly. The standalone list is ordered from basic API use
toward advanced escape hatches:

```bash
cargo run --manifest-path samples/basic-queries/Cargo.toml
cargo run --manifest-path samples/write-dtos/Cargo.toml
cargo run --manifest-path samples/generated-schema/Cargo.toml
cargo run --manifest-path samples/json-search/Cargo.toml
cargo run --manifest-path samples/joins-and-aggregates/Cargo.toml
cargo run --manifest-path samples/transactions/Cargo.toml
cargo run --manifest-path samples/error-handling/Cargo.toml
cargo run --manifest-path samples/numeric/Cargo.toml
cargo run --manifest-path samples/custom-types/Cargo.toml
cargo run --manifest-path samples/postgres-types/Cargo.toml
cargo run --manifest-path samples/cte-and-subqueries/Cargo.toml
cargo run --manifest-path samples/advanced-queries/Cargo.toml
cargo run --manifest-path samples/raw-query/Cargo.toml
```

`samples/rest-api` is the larger application sample to read after the standalone examples. The other directories each focus on one rqb use case.
