# Roadmap

These are useful follow-ups that came out of the REST sample and API review. They are not required for the current API to work, but they are good candidates for the next round of ergonomics work.

## Aggregate Filters

`json_agg("orders", fields)` returns an empty array instead of `null`, but left joins still need an explicit aggregate filter such as:

```rust
.filter_agg("orders", order.id().is_not_null())
```

A cleaner API would let the aggregate builder accept the filter directly, or infer a non-null marker field for common left-join JSON aggregate cases.

## Generated Validators

The CLI emits Postgres enum metadata and serde-compatible Rust enum wrappers. Fixed-shape DTOs can use those generated enums directly.

Request DTOs that deliberately keep enum fields as strings still need application validation. A future generator pass could emit reusable validation helpers for those string-backed inputs.

## Dynamic Response Shapes

Endpoints that let clients choose arbitrary fields should keep returning `serde_json::Value`, because the response shape is dynamic by design.

The docs should keep drawing a clear line between typed fixed-shape endpoints and dynamic search endpoints.

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

## Validated Expression Reuse

Validation now carries resolved SELECT and write shapes into the renderer, including CTE/subquery bodies and write filters. The remaining work is to make type/operator metadata easier to centralize and to keep future query features from reintroducing raw AST reads in rendering.

A future refactor should introduce validated expression nodes that carry `ResolvedField` data through rendering:

- preserve already resolved fields for predicates, column predicates, and subqueries
- keep SQL rendering a mostly mechanical pass over validated data
- keep validated write models render-ready without depending on raw write AST details

This should be an internal architecture change, not a public ergonomics regression.
