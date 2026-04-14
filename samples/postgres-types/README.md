# Postgres Types

Shows generated metadata and serde row mapping for common Postgres-specific
types: `citext`, `bytea`, `inet`, `cidr`, ranges, timestamp, and timestamptz.

Run from the repository root after `make db-up`:

```bash
cargo run --manifest-path samples/postgres-types/Cargo.toml
```
