# Write DTOs

Shows `WriteRecord` DTO mapping for inserts and patch updates:

- `#[rqb(field = ...)]` for DTO names that differ from generated fields
- `#[rqb(skip)]` for request-only fields
- `#[rqb(skip_none)]` for patch DTOs
- `#[rqb(json)]` for nested JSONB structs

Run from the repository root after `make db-up`:

```bash
cargo run --manifest-path samples/write-dtos/Cargo.toml
```
