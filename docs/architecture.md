# Architecture

This document describes what rqb currently does and what the internal architecture
should optimize for while the API is still pre-public.

## Capability Spec

rqb is a Postgres-first runtime query builder. Application Rust code owns the
trusted query shape; client JSON requests can only refine that shape through
metadata-constrained fields, filters, sorting, limit, and offset.

Current public capabilities:

- Dataset metadata for tables, views, CTE sources, and raw server-owned sources.
- Field metadata for API name, DB name, type, capabilities, JSON path policy,
  text search configuration, enum values, custom domain metadata, and generated
  relation helpers.
- SELECT queries with default root projection, explicit fields, joins, CTEs,
  raw sources, subqueries, `EXISTS`, `DISTINCT`, `DISTINCT ON`, grouping,
  aggregate selection, aggregate filters, row locks, sorting, pagination, and
  JSON `SearchRequest` merge.
- Expression trees for predicates, column predicates, logical `and`/`or`/`not`,
  raw server-owned fragments, subquery predicates, and exists predicates.
- Operators for scalar comparisons, null checks, text matching, regex, arrays,
  JSONB keys and array matching, text search, ranges, networks, and column
  comparisons.
- INSERT/UPDATE/DELETE with serde-backed values, raw assignments, column
  assignments, `RETURNING`, `ON CONFLICT`, `INSERT ... SELECT`, and required
  DELETE filters.
- Raw top-level SQL through `raw_query`, with bind count validation, `?`
  placeholder rendering, `??` escaping, raw row mapping, scalar fetch helpers,
  and statement-cache bypass.
- Postgres rendering with `$N` parameters, typed casts, exact numeric binding
  through text for `numeric` and decimal-string domains, stable `ANY($1)` shape
  for `IN`, and cache policy in `BuiltQuery`.
- Runtime execution through `PgExecutor` implementations for clients, pool
  clients, transactions, pooled `Db`, savepoints, and page helpers.
- CLI introspection and schema generation for fields, enums, domains, generated
  Rust enum wrappers, relation helpers, JSONB policy, arrays, ranges, and common
  Postgres types.

## Current Pipeline

The intended flow is:

```text
builder / JSON request
-> AST query structs
-> validation and field resolution
-> validated query structs
-> Postgres rendering
-> optional execution / row mapping
```

`rqb-core` owns metadata, ASTs, validation, and validated models. It has no
Postgres runtime dependencies.

`rqb-postgres` owns SQL rendering, parameter conversion, row mapping, executor
traits, pool and transaction helpers.

`rqb-cli` owns database introspection and generated schema shape.

## Architecture Rules

Validation owns correctness. Rendering should be a mostly mechanical pass over
validated data.

Values are parameters. Raw SQL is only trusted server-owned SQL, and bind counts
must be validated before rendering.

Metadata is the contract between Rust builders, JSON requests, validation,
rendering, execution, row mapping, generated code, samples, and docs.

Extensibility should be boring. Adding a library-owned Postgres type or a
project-specific domain should have an obvious path through metadata, validation,
casts, params, row mapping, CLI, tests, and docs.

## Pressure Points

### Type Knowledge Is Scattered

Type behavior currently lives in several places:

- `FieldType` / `ElemType` methods in `rqb-core`
- value validation in `validation/operators.rs`
- Postgres cast helpers in `rqb-postgres/src/type_sql.rs`
- parameter conversion in `params.rs`
- row mapping in `row_map.rs`
- CLI catalog introspection, type mapping, and code generation modules

This is correct behaviorally, but adding a new type still requires touching
several files. The target is not a dynamic registry yet. Type classification and
Postgres cast/selection behavior now have dedicated modules; future type work
should keep tightening that checklist instead of scattering new switch arms into
generic helpers.

### Operator Semantics Are Parallel Switches

Validation and rendering both switch over `Operator` and field type shape. That
keeps SQL rendering simple, but it means a new operator requires careful edits in
both layers.

The target is not to hide SQL behind a generic trait maze. The target is to make
operator categories explicit: scalar comparison, text match, array membership,
JSONB key, range/network containment, regex, text search, and subquery.

### Write Validation Still Reuses Select Machinery

Write filters and write field resolution currently create small throwaway
`SelectQuery` / `QueryScope` values to reuse field resolution. That works, but it
is the wrong abstraction. Writes need a dedicated scope over one writable
dataset.

Validated write structs should only contain data the renderer needs. They should
not keep the original write AST.

### Execution Surface Is Wider Than The Conceptual Model

The user sees query helpers through prelude, so the ergonomics are acceptable,
but internally there are separate execution traits for select, write, and raw
queries. `PgExecutor` also exposes cached and uncached methods because cache
policy lives in `BuiltQuery`.

This should stay stable until a refactor clearly reduces code without making raw
cache bypass or pool/client/transaction support less explicit.

### Row Mapping Has Two Paths

Normal rqb queries map rows using validated `SelectColumn` metadata. Raw queries
map rows from Postgres column type OIDs. Both currently go through a JSON bridge
before serde deserialization.

That is acceptable for current API endpoints. Direct row deserialization is a
later performance project, not the first architecture slice.

### CLI Is Product-Critical

CLI internals are split by responsibility: catalog introspection, type mapping,
generated code rendering, identifier hygiene, and shared schema models. The
generated output is product-critical, so future changes should keep those
boundaries crisp and verify the golden schema.

## Target Shape

The target architecture keeps the public API simple and makes internals less
coupled:

```text
rqb-core
  metadata: Dataset, Source, Field, FieldType, TypeSpec, capabilities
  ast: SelectQuery, SearchRequest, Expr, Aggregate, write ASTs, RawSql
  scope: field and qualifier resolution
  validate: AST -> validated models
  validated: render-ready resolved structs

rqb-postgres
  type_sql: Postgres casts, selection repr, array casts
  render: validated models -> BuiltQuery
  render::params: Value -> SQL placeholder and cast shape
  params: Value -> ToSql-owned params
  rows: Row -> serde bridge
  exec: PgExecutor and high-level fetch helpers

rqb-cli
  introspect: read Postgres catalog
  type_map: Postgres catalog type -> rqb metadata
  render: generated schema Rust code
```

Names can change. The important boundary is that rendering should not ask
unvalidated AST questions, and type/operator rules should not be rediscovered in
every layer.

## Migration Plan

1. Clean validated write models. Done.
   Store the writable dataset and validated fields directly; do not store the
   original write AST in render inputs.

2. Add a write-specific scope. Started.
   Replace throwaway `SelectQuery` construction in write validation with a small
   `WriteScope` or single-dataset resolver.

3. Centralize type behavior. Started.
   Introduce small helper APIs that answer type-family, value shape, element
   type, Postgres cast, selection representation, and array cast questions from
   one place per layer.

4. Categorize operators.
   Keep `Operator` as the JSON/user-facing enum, but route validation and
   rendering through explicit operator categories to reduce parallel branching.

5. Revisit execution traits.
   Only after rendering and validation are cleaner, decide whether select/write
   and raw execution helpers can share implementation without hiding semantics.

6. Split CLI internals. Done.
   Catalog introspection, type mapping, code rendering, identifier hygiene, and
   shared schema models now live in separate modules.

## Non-Goals For This Refactor

- Do not introduce a Diesel-like compile-time type system.
- Do not add `Box<dyn ToSql>` to normal query code.
- Do not make rendering validate user-facing semantics again.
- Do not hide raw SQL safety rules behind convenience APIs.
- Do not preserve awkward API solely for compatibility before public release.
