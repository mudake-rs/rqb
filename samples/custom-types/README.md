# custom-types

Shows a project-specific Postgres domain generated as `TypeSpec`.

The fixture defines `uint_256` as an exact numeric domain. The generated schema
maps it to `FieldType::Custom`, binds values as decimal strings, and selects
them back as strings so precision is never lost.

Run from the repository root:

```bash
make db-up
cargo run --manifest-path samples/custom-types/Cargo.toml
```
