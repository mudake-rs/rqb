# Raw Query

Shows raw SQL binds, escaped question marks, raw sources, and raw predicates
without giving up bind-count validation.

Execution mode: renders SQL and asserts it. No database connection is opened.

## What This Shows

- `raw("... ? ...").bind(value)` renders `$N` placeholders in order.
- `raw_expr(...)` plugs a server-owned expression into a typed projection
  without a manual `ValueExpr::Raw` struct literal.
- `raw_predicate(...)` plugs a server-owned predicate into a typed query without
  a manual `BoolExpr::Raw` struct literal.
- Placeholder scanning skips quoted strings, dollar-quoted bodies, quoted
  identifiers, and comments.
- `??` renders a literal question mark outside those SQL contexts without
  consuming a bind.
- Raw sources still declare exposed fields so outer typed queries know what can
  be projected or filtered.
- Bind-count mismatches are validation errors, not Postgres runtime surprises.
- Raw SQL is server-owned; client JSON never supplies raw fragments.
- Any raw fragment disables persistent prepared-statement caching for the built
  query. Prefer typed DSL helpers when they cover the same SQL shape.

Run with:

```bash
cargo run --manifest-path samples/raw-query/Cargo.toml
```
