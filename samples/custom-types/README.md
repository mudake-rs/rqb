# Custom Types

Shows unknown Postgres types as raw-only schema metadata. They stay usable in
server-owned raw SQL while typed `Field<T>` constants are generated only for
known sqlx-supported types.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- Unknown extension types still appear in default projection metadata.
- Raw-only metadata can become a value expression wherever `Into<ValueExpr>` is
  accepted.
- Custom operators such as pgvector distance stay parameterized through
  `ValueExpr::op(...)`.
- Raw-only `tsvector` metadata can participate in full-text `@@` predicates and
  ranking helpers.
- Unsupported types stay hidden from JSON search unless a typed JSON shape is
  explicitly added later.

Run with:

```bash
cargo run --manifest-path samples/custom-types/Cargo.toml
```
