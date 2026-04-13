# generated-schema

Shows the intended CLI flow: generate schema metadata once in `samples/sample-base`, then use generated fields, enums, domains, and relation helpers from sample code.

Regenerate the shared sample schema from the repository root:

```bash
make db-up
make generate-sample-base-schema
cargo run --manifest-path samples/generated-schema/Cargo.toml
```
