# raw-query

Shows `raw_query()` as the top-level SQL escape hatch: bind parameters,
`fetch_as` by column names, scalar reads, escaped question marks, and execution
inside an explicit transaction.

Run from the repository root:

```bash
make db-up
cargo run --manifest-path samples/raw-query/Cargo.toml
```
