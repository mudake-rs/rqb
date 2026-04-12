# Roadmap

These are useful follow-ups that came out of the REST sample and API review. They are not required for the current API to work, but they are good candidates for the next round of ergonomics work.

## Aggregate Filters

`json_agg("orders", fields)` returns an empty array instead of `null`, but left joins still need an explicit aggregate filter such as:

```rust
.filter_agg("orders", order.id().is_not_null())
```

A cleaner API would let the aggregate builder accept the filter directly, or infer a non-null marker field for common left-join JSON aggregate cases.

## Generated Validators

The CLI now emits Postgres enum metadata and Rust enum wrappers. Request DTO validation still lives in the application through small `validator` custom functions.

A future generator pass could emit reusable validation helpers for enum-backed request strings, or make it easier to use generated Rust enums directly in API payloads.

## Dynamic Response Shapes

Endpoints that let clients choose arbitrary fields should keep returning `serde_json::Value`, because the response shape is dynamic by design.

The docs should keep drawing a clear line between typed fixed-shape endpoints and dynamic search endpoints.

## Web Extractors

The sample calls `payload.validate()?` in handlers. That is explicit and easy to follow, but Actix users may want a small extractor wrapper that validates `Json<T>` and `Query<T>` automatically.

This belongs in sample/application code rather than core rqb.

## Extensible Type Metadata

`PHILOSOPHY.md` describes the target type model: rqb owns broad Postgres and common extension type support, while project-specific domains and scalars are declared through metadata.

This is not fully implemented yet. The current type model is still mostly a closed `FieldType` / `ElemType` / `Value` set. Before beta, add a vertical slice for exact numeric/domain support:

- represent exact `numeric` without silently converting through `f64`
- introspect Postgres domains in the CLI instead of flattening them to text or their base type
- generate project-local type metadata for domains such as `uint_256`
- validate domain/custom scalar input through metadata, including JSON `SearchRequest`
- render lossless casts and selection output for exact values
- keep built-in Postgres types on fast paths

The BFM `uint_256` domain is the reference use case for this work.

## Validated Expression Reuse

Validation currently resolves some field references again during rendering paths such as CTE rendering, subqueries, write filters, and raw write conflict filters. This is correct behaviorally, but it is unnecessary repeated work and makes future type/operator metadata harder to centralize.

A future refactor should introduce validated expression nodes that carry `ResolvedField` data through rendering:

- validate CTE and subquery bodies once
- render write filters without constructing throwaway `ValidatedSelect` values
- preserve already resolved fields for predicates, column predicates, and subqueries
- keep SQL rendering a mostly mechanical pass over validated data

This should be an internal architecture change, not a public ergonomics regression.
