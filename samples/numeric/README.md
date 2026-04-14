# Numeric

Shows exact numeric transport for a Postgres `numeric` domain generated as an
rqb custom type. The sample keeps large values as decimal strings instead of
rounding through `f64`.

Run from the repository root after `make db-up`:

```bash
cargo run --manifest-path samples/numeric/Cargo.toml
```
