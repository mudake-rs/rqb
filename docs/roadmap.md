# Roadmap

This is the working roadmap for rqb before a public beta. The project is still
pre-release, so API compatibility is not sacred. Correctness, clarity, and
ergonomics win over preserving awkward names or internal shapes.

## Current Baseline

rqb is a Postgres-only runtime query builder for services that need both
server-owned Rust query composition and metadata-constrained JSON search.

The current baseline includes:

- Postgres metadata for tables, views, CTEs, raw sources, subquery sources,
  generated relations, fields, enums, ranges, domains, and custom type specs.
- SELECT builders with joins, lateral joins, CTEs, subqueries, `EXISTS`,
  `DISTINCT ON`, grouping, aggregates, row locks, set operations, computed
  select items, typed functions, JSON expression accessors, and window
  functions.
- JSON `SearchRequest` for safe client filters, sorting, field selection,
  limit, and offset on metadata-declared fields.
- INSERT, UPDATE, DELETE, upsert, `RETURNING`, expression assignments,
  `UPDATE ... FROM`, `DELETE ... USING`, and required DELETE filters.
- Raw SQL escape hatch through `raw_query`, with bind count validation and
  scalar/typed fetch helpers.
- Postgres execution through `PgExecutor`, pooled `Db`, `Tx`, savepoints,
  linear transactions, closure transactions, pages, and statement cache policy.
- Structured Postgres errors for common constraint failures, retryable
  transaction failures, cancellation, privilege errors, raw SQLSTATE access, and
  table/column/detail/hint extractors.
- CLI schema generation from live Postgres, including enums, domains, relation
  helpers, arrays, JSONB policy, ranges, and common Postgres types.
- Docker test flow, integration tests against Postgres, CLI introspection golden
  tests, docs, recipes, and samples.

## Project Decisions

These decisions should guide feature work and reviews:

- rqb is Postgres-only. Do not add dialect abstractions, generic SQL backends, or
  lowest-common-denominator APIs.
- Validation owns semantics. Rendering should be mechanical over validated
  models, not a second place where operator/type meaning is rediscovered.
- Metadata is the contract between Rust builders, JSON search, rendering, row
  mapping, CLI generation, docs, and tests.
- Rust/server code may build powerful SQL: joins, CTEs, raw fragments,
  expressions, windows, and custom sources. JSON clients stay constrained to
  declared fields and safe filter/sort/page operations.
- Exact numbers must stay exact. `Float` means Postgres `double precision`.
  `Numeric` and decimal-string domains use lossless string-backed transport by
  default. See `docs/numeric-policy.md`.
- API names can break before beta. If a name is misleading, fix it now.
- Every behavior change should bring validation tests, rendering tests, and an
  integration test when Postgres behavior is involved.

## P0: Before Beta

P0 items are correctness or API-shape work that should be done before we try to
make the library broadly consumable.

### Numeric Correctness

Done:

- `FieldType::Integer` and `ElemType::Int` reject values outside the Postgres
  `int4` range before rendering.
- Typed Postgres bind params avoid noisy `::bigint::int` casts for integer
  values and typed nulls.
- `Numeric` and decimal-string custom domains bind and select through text so
  precision is not lost.
- Exact numeric fields reject implicit `f64` values.
- Expression promotion rejects `Numeric + Float` and preserves custom numeric
  domains for integer fallback values.
- `sum` and `avg` preserve exact output for integer, bigint, numeric, and
  decimal-string domain inputs.
- `u64` inputs convert to `I64` when they fit and to decimal strings when they
  do not, avoiding f64 precision loss.
- The public numeric policy is documented in `docs/numeric-policy.md`.

Remaining P0 work:

- Keep adding numeric regression tests when new expression, operator, aggregate,
  write, and custom type paths are added.

### API Naming And Facade Cleanup

Done:

- Write conflict filters use `conflict_filter`, not the general SELECT-style
  `filter` name.
- `fetch_all_as`, `fetch_one_as`, and `fetch_optional_as` make cardinality
  explicit across select, write, and raw execution.
- `rqb::Error` and `rqb::Result` point at the facade-level Postgres error path;
  core validation errors are available as `rqb::CoreError`.
- Text substring matching uses `contains`; range/network containment uses
  `covers`, `contained_by`, and `overlaps`.
- Array scalar membership uses `contains_element` and
  `not_contains_element`.
- The facade exports are explicit; internal validated models do not leak through
  `rqb::*`.

Ongoing:

- Keep checking naming when new operators are added.

### JSON Search API Shape

Done:

- Request JSON uses `filter`, not `query`.
- Sort directions deserialize as lowercase `asc` / `desc`.
- JSON SearchRequest remains limited to fields, filter, sort, limit, and offset.

Break and clean the JSON DSL before beta:

- Make logical expression shape consistent and easy to produce from clients.
- Keep JSON SearchRequest away from joins, CTEs, raw SQL, computed aliases, and
  arbitrary expressions.
- Put SearchRequest serde support behind a default-on feature if it materially
  helps users who only want the Rust builder.

### Error Ergonomics

Done:

- `SerializationFailure` and `DeadlockDetected` for retry loops.
- `QueryCanceled` for statement timeouts and cancellations.
- `InsufficientPrivilege` for RLS and permission failures.
- `RestrictViolation` completes the important constraint family.
- `is_retryable()`, raw SQLSTATE `code()`, `table_name()`, `column_name()`,
  `detail()`, and `hint()` are available.
- Single-variant `is_*` helpers were pruned where they did not carry real
  ergonomic value.
- Facade-level `rqb::Error` and `rqb::Result<T>` are the normal application
  error path.

Remaining P0 work:

- Show retryable error handling and constraint mapping in docs and samples.

### Docs And Samples

The docs should be copyable, not just descriptive:

- Rewrite the README around a short hero example, rendered SQL, JSON search, and
  server-owned queries.
- Add rendered SQL blocks to guide and recipes where they clarify behavior.
- Add crate-level docs for docs.rs.
- Keep samples on one generated sample schema instead of repeating hand-written
  schemas.
- Preserve standalone examples for basic CRUD, JSON search, joins and
  aggregates, transactions, CTEs/subqueries, generated schema, raw query, custom
  types, and error handling.
- Show both transaction styles: explicit `begin` / `commit` and closure-style
  transaction.
- Include a custom type sample based on a decimal-string domain such as
  `uint_256`.
- Include manual complex query examples using `and`, `or`, `not`, subqueries,
  expressions, and typed result mapping, including selecting only IDs.

### Test Coverage As Spec

Tests should function as the executable spec:

- Every operator has validation and rendering coverage.
- Every type has read/write/filter coverage where the database semantics matter.
- Every public builder method has at least one behavior test.
- Every error variant that claims to be reachable has an integration or unit
  test proving how it is reached.
- CLI generation has golden tests against live Postgres for enums, domains,
  arrays, ranges, custom types, and relation helpers.
- `make docker-test` remains the one command that brings up Postgres and runs
  the full test suite.

## P1: Postgres Depth

P1 is the next feature layer after the P0 correctness/API pass.

### Type Coverage

Already covered well:

- UUID, booleans, integer/bigint/float/numeric, text, citext, JSONB, bytea,
  timestamp/timestamptz/date, arrays, enums, ranges, inet/cidr, and custom
  domains through `TypeSpec`.

Add next:

- `time`, `timetz`, and `interval`.
- `json` in addition to `jsonb`, with honest operator limitations.
- `macaddr` and `macaddr8`.
- `tsvector` and `tsquery` for indexed full-text search columns.
- Multirange types for PG 14+.
- `bit` and `varbit`.
- `hstore` and `ltree` as common extensions.
- `pgvector` through type metadata and focused operators rather than a giant
  hardcoded surface.
- PostGIS later, after the extension-type story is settled.

### Operators

Add missing Postgres operators where they are common and safe:

- Case-sensitive LIKE and regex variants.
- JSONB containment and contained-by for arbitrary JSON values.
- PG 12 JSON path operators `@?` and `@@`.
- Range adjacency, left/right, union, intersection, and difference where the API
  can stay clear.
- Network strict containment and overlap operators.
- `IS TRUE`, `IS FALSE`, and `IS UNKNOWN` for nullable boolean fields.
- Array subscript and expression-level indexing where it fits the expression
  model.

### Expressions And Functions

The expression core is in place. Continue expanding it through typed helpers:

- Finish core function helpers: `concat`, `replace`, `regexp_replace`,
  `split_part`, `extract`, `age`, `abs`, `ceil`, `floor`, `round`, `power`, and
  `sqrt`.
- Add JSON functions such as `jsonb_set`, `jsonb_build_object`, and
  `jsonb_array_elements` where result typing is clear.
- Add array functions such as `array_length`, `unnest`, `array_append`, and
  `array_remove`.
- Provide an explicit generic `func()` escape hatch that requires a return type.
- Keep computed aliases server-owned. JSON SearchRequest should not filter on
  computed aliases unless they are exposed through dataset metadata, usually via
  a view.

### Aggregates And Analytics

- Make aggregate output types exact and metadata-aware.
- Add ordered-set aggregates such as `percentile_cont`, `percentile_disc`, and
  `mode`.
- Add aggregate window usage where it shares the existing window model cleanly.
- Add window frame specs after the current function/partition/order model is
  stable.
- Add `GROUPING SETS`, `CUBE`, and `ROLLUP` if the API can stay readable.

### Sources And Query Shapes

The current architecture supports query bodies and set operations. Remaining
source work:

- `VALUES` as a table source.
- Function calls as table sources.
- Materialized and not-materialized CTE hints.
- Scalar subqueries in SELECT expressions where they share the expression model.
- ANY/ALL/SOME with subqueries beyond `IN` / `NOT IN`.

### Writes And Bulk Work

- Custom `ON CONFLICT ... WHERE` for partial unique indexes.
- Richer `ON CONFLICT DO UPDATE SET` expressions.
- CTEs in write queries.
- Batch update through `VALUES`.
- Auto-chunked multi-row insert.
- COPY FROM STDIN for high-volume ingest.

## P2: Performance And Polish

These are valuable but should follow correctness and API shape:

- Direct row deserialization without the intermediate JSON map.
- Benchmarks for rendering, execution overhead, row mapping, raw query, and
  common API endpoints.
- Smaller allocation passes in rendering and row mapping when they are proven by
  benchmarks.
- Optional decimal crate integrations, while keeping string-backed exact numeric
  transport as the default.
- Optional LRU statement-cache policy only if the current per-connection cache
  becomes a measured problem.

## Release Readiness

Before publishing broadly:

- README and docs must match the current API.
- Samples must run against the generated sample schema.
- `cargo test --workspace --all-features`, no-default-features checks, clippy,
  rustdoc, examples, and `make docker-test` must pass.
- License, repository metadata, and contribution instructions must be clean.
- The public API can still be pre-1.0, but the design should be internally
  coherent enough that new features do not require another architecture cleanup.

## Non-Goals

- rqb is not an ORM. No identity map, association loader, model lifecycle, or
  migration system.
- rqb is not Diesel. It does not try to encode every SQL expression in Rust's
  compile-time type system.
- rqb is not sqlx. Raw SQL is supported, but the primary value is metadata-driven
  query construction and validation.
- rqb will not accept arbitrary SQL from JSON clients.
- rqb will not make `BigDecimal` or `rust_decimal` mandatory for exact numeric
  transport.
- rqb will not preserve awkward pre-beta APIs for compatibility.
