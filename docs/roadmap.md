# Roadmap

These are useful follow-ups that came out of the REST sample and API review. They are not required for the current API to work, but they are good candidates for the next round of ergonomics work.

## Aggregate Modifiers

Inline aggregate modifiers are available:

```rust
.agg(
    json_agg("orders", [order.id(), order.status()])
        .filter(order.id().is_not_null())
        .order_by(order.created_at().desc())
)
```

Alias-based `filter_agg` and `order_within` remain useful for builder-style
composition, but examples should prefer inline modifiers when the aggregate is
created in the same expression.

## Generated Validators

The CLI emits Postgres enum metadata and serde-compatible Rust enum wrappers. Fixed-shape DTOs can use those generated enums directly.

Request DTOs that deliberately keep enum fields as strings still need application validation. A future generator pass could emit reusable validation helpers for those string-backed inputs.

## Dynamic Response Shapes

Endpoints that let clients choose arbitrary fields should keep returning `serde_json::Value`, because the response shape is dynamic by design.

The docs should keep drawing a clear line between typed fixed-shape endpoints and dynamic search endpoints.

## Numeric Correctness

`docs/numeric-policy.md` documents the target rule: `Float` is lossy
`double precision`; `Numeric` and numeric-like domains use exact string-backed
transport by default.

Done:

- `FieldType::Integer` and `ElemType::Int` reject values outside PostgreSQL
  `int4` range before rendering.
- Postgres rendering lowers integer metadata to typed bind params:
  `BindParam::Int4` / `BindParam::Int4Array` with `::int` / `::int[]`.
- Numeric and decimal-string domains bind through text and select as text.

Open before beta:

- `compatible_type` must stop promoting `Numeric + Float` to `Float`.
- custom numeric domains must not lose identity during expression promotion.
- `sum` and `avg` must preserve exact output for numeric and numeric-like inputs.
- decide whether implicit `F64` values are rejected for `Numeric` fields or
  allowed only through an explicit cast/escape hatch.
- add `Value::from(u64)` ergonomics without precision loss.

## Web Extractors

The sample calls `payload.validate()?` in handlers. That is explicit and easy to follow, but Actix users may want a small extractor wrapper that validates `Json<T>` and `Query<T>` automatically.

This belongs in sample/application code rather than core rqb.

## Extensible Type Metadata

`PHILOSOPHY.md` describes the target type model: rqb owns broad Postgres and common extension type support, while project-specific domains and scalars are declared through metadata.

The first domain vertical slice is implemented: `TypeSpec` metadata, `FieldType::Custom`, `ElemType::Custom`, CLI domain introspection, exact decimal-string binding for scalar and array domains, text selection for exact numeric fields, and runtime tests against a `uint_256` domain.

Remaining work before beta:

- richer per-domain validation rules beyond decimal shape
- serde-friendly generated Rust newtypes for common exact domains
- exact aggregate output for `sum` / `avg` when the result is numeric
- richer range/network operator coverage beyond `contains`, `contained_by`, and `overlaps`
- library-owned extension types such as `ltree`, `hstore`, `macaddr`, `bit`/`varbit`, and interval
- `pgvector` and PostGIS through `TypeSpec`/custom operator metadata rather than a large hardcoded surface

The BFM `uint_256` domain is the reference use case for this work.

## Architecture Refactor Status

The current architecture refactor is complete enough for the next feature work:

- validation carries resolved SELECT and write shapes into the renderer
- write validation uses a write-specific scope instead of throwaway selects
- operators lower into concrete validated predicate shapes before rendering
- type classification, array metadata, and custom representation helpers are centralized in the type model
- Postgres cast, selection, and type-name behavior has a dedicated `type_sql` module
- the public facade hides internal validated structs while `rqb-postgres` can still consume them

Future query features should preserve this shape: validation decides semantics,
rendering stays mechanical, and new type/operator behavior goes through the
metadata contract instead of scattered renderer checks.
