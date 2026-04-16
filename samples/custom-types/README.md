# Custom Types

Shows unknown Postgres types as raw-only schema metadata. They stay usable in
server-owned raw SQL while typed `Field<T>` constants are generated only for
known sqlx-supported types.

Run with:

```bash
cargo run --manifest-path samples/custom-types/Cargo.toml
```
