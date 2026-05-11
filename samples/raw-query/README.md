# Raw Query

Shows raw SQL binds, escaped question marks, raw sources, and raw predicates
without giving up bind-count validation.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- `raw("... ? ...").bind(value)` renders `$N` placeholders in order.
- `raw_expr(...)` and `raw_predicate(...)` plug server-owned fragments into
  typed projections and filters without manual struct literals.
- `??` renders a literal question mark without consuming a bind.
- Raw sources still declare exposed fields so outer typed queries know what can
  be projected or filtered.
- Bind-count mismatches are validation errors, not Postgres runtime surprises.
- Raw SQL is server-owned; client JSON never supplies raw fragments.

Run with:

```bash
cargo run --manifest-path samples/raw-query/Cargo.toml
```
