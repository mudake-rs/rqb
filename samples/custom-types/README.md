# Custom Types

Shows raw-only extension metadata and a working TOML type mapping for a
PostgreSQL domain. `types::Cents` uses sqlx's transparent bigint codec; the
domain owns its nonnegative constraint. TOML chooses the Rust path and search
capabilities, not value conversion.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- Unknown extension types still appear in default projection metadata.
- Raw-only metadata can become a value expression with `*_META.expr()` or
  `*_META.at("alias")` wherever `Into<ValueExpr>` is accepted.
- Custom operators such as pgvector distance stay parameterized through
  `ValueExpr::op(...)`.
- Raw-only `tsvector` metadata can participate in full-text `@@` predicates,
  phrase queries, and ranking helpers.
- Unsupported types stay hidden from JSON search unless a typed JSON shape is
  explicitly added later.

Run with:

```bash
cargo run --manifest-path samples/custom-types/Cargo.toml
```

The separate `mapped_schema.rs` intentionally tests a different schema from
the shared examples. Regenerate it from the repository root:

```bash
make db-up
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f samples/custom-types/schema.sql
cargo run -p rqb-cli -- generate --database-url "$DATABASE_URL" --schema sample_custom \
  --config samples/custom-types/rqb.toml --out samples/custom-types/src/mapped_schema.rs
RQB_TEST_DATABASE_URL="$DATABASE_URL" cargo test --manifest-path samples/custom-types/Cargo.toml -- --ignored
```

`json = "big_int"` means the search DTO binds an integer compatible with this
domain. A custom codec alone does not define a JSON search representation.
