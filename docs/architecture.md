# Architecture

This document describes what rqb currently does and what the internal architecture
should optimize for while the API is still pre-public.

## Capability Spec

rqb is a Postgres-first runtime query builder. Application Rust code owns the
trusted query shape; client JSON requests can only refine that shape through
metadata-constrained fields, filters, sorting, limit, and offset.

Current public capabilities:

- Dataset metadata for tables, views, CTE sources, raw server-owned sources,
  and validated subquery sources.
- Field metadata for API name, DB name, type, capabilities, JSON path policy,
  text search configuration, enum values, custom domain metadata, and generated
  relation helpers.
- SELECT queries with default root projection, explicit fields, joins,
  `LATERAL` joins, CTEs, raw sources, subquery sources, subqueries, `EXISTS`,
  `DISTINCT`, `DISTINCT ON`, grouping, aggregate selection, expression select
  items, aggregate filters, row locks, sorting, pagination, and JSON
  `SearchRequest` merge.
- Query body composition through `QueryExpr`, including `UNION`, `UNION ALL`,
  `INTERSECT`, and `EXCEPT` with validated output column count, output type
  compatibility, final ordering, limit, and offset.
- Expression trees for predicates, column predicates, logical `and`/`or`/`not`,
  raw server-owned fragments, subquery predicates, and exists predicates.
- Server-owned SQL value expressions for computed select items, including
  fields, values, raw fragments, typed function calls, `COALESCE`, searched
  `CASE`, and casts.
- Operators for scalar comparisons, null checks, text matching, regex, arrays,
  JSONB keys and array matching, text search, ranges, networks, and column
  comparisons.
- INSERT/UPDATE/DELETE with serde-backed values, raw assignments, column
  assignments, expression assignments, `RETURNING` expressions, `ON CONFLICT`,
  custom conflict assignments, `UPDATE ... FROM`, `DELETE ... USING`,
  `INSERT ... SELECT`, and required DELETE filters.
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

## Postgres-Only Modularity

rqb is Postgres-only. The crate split is not a promise of future MySQL, SQLite,
or generic SQL dialect support.

Keep modules when they separate real responsibilities:

- `rqb-core`: runtime-free metadata, ASTs, JSON request types, validation, and
  validated models
- `rqb-postgres`: Postgres SQL rendering, casts, params, row mapping, execution,
  pools, transactions, and savepoints
- `rqb-cli`: Postgres catalog introspection and generated schema code
- `rqb`: facade and prelude

Do not add `Backend`, `Dialect`, generic renderer, or generic executor layers
without a concrete current need. The architecture should expose Postgres clearly
instead of hiding it behind lowest-common-denominator abstractions.

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

Numeric correctness is part of the metadata contract. `Float` means Postgres
`double precision` and may use `f64`; `Numeric` and decimal-string domains mean
exact transport and must not silently pass through `f64`. The default exact path
is text-backed binding/selection, not a mandatory `BigDecimal` or
`rust_decimal` dependency. See `docs/numeric-policy.md`.

## Pressure Points

### Type Knowledge Is Scattered, But Bounded

Type behavior currently lives in several places:

- `FieldType` / `ElemType` methods in `rqb-core`
- value validation in `validation/operators.rs`
- Postgres cast helpers in `rqb-postgres/src/type_sql/{casts,names,selection}.rs`
- SQL placeholder/cast rendering in `render/params.rs`
- runtime parameter conversion in `params.rs`
- row mapping in `row_map/{typed,raw,values}.rs`
- CLI catalog introspection, type mapping, and code generation modules

This is correct behaviorally, but adding a new type still requires touching
several files. The target is not a dynamic registry yet. Type classification and
array element metadata live on `FieldType`; representation helpers live on
`ValueRepr`, `SelectRepr`, and `TypeSpec`; Postgres cast/selection behavior now
has dedicated modules. Future type work should keep tightening that checklist
instead of scattering new switch arms into generic helpers.

### Operator Semantics Are Lowered In Validation

`Operator` remains the user-facing and JSON-facing enum, but rendering no longer
interprets `Operator + FieldType + Value` directly. Validation routes operators
through `OperatorCategory`, validates the field/type/value combination, and
lowers each leaf into a concrete `ValidatedPredicate` shape.

The current expression model is:

```text
ValidatedExpr
  Predicate(ValidatedPredicate)
  Logical { and/or/not, predicates }
```

`ValidatedPredicate` owns value predicates, column comparisons, subqueries,
`EXISTS`, and raw server-owned fragments. This keeps `render::expr` focused on
logical composition, while `render::predicate` mechanically renders concrete
predicate shapes.

Text matching and containment are deliberately separate API concepts. Rust
methods `contains` / `not_contains` and JSON operators `contains` /
`notContains` lower only to text-like `LIKE` predicates. Range and network
containment use Rust methods `covers` / `not_covers`, `contained_by`, and
`overlaps`, with matching JSON operators `covers`, `notCovers`, `containedBy`,
and `overlaps`. The renderer only sees the concrete lowered predicate shape.

### SQL Value Expressions Are Server-Owned

`SqlExpr` is the Rust/server-owned expression layer. It is separate from JSON
`Expr` on purpose:

```text
Expr    = metadata-constrained boolean predicates, serde-facing
SqlExpr = trusted value expressions for SELECT items, write assignments, and RETURNING items
```

Computed select and returning expressions lower into `ValidatedSelectItem`
values carrying a validated expression, explicit alias, and output type. Row
mapping consumes the same output metadata, so computed aliases deserialize like
ordinary fields.

JSON `SearchRequest` does not see computed select aliases. If an expression must
be client-addressable, expose it as dataset metadata, usually through a view or
generated field.

### Query Bodies Own Set Operations

`QueryExpr` is the common server-owned query body used by top-level reads, CTEs,
subquery predicates, `EXISTS`, and `INSERT ... SELECT`. A query body is either a
plain `SelectQuery` or a `SetQuery`.

Validation lowers `SetQuery` into `ValidatedSetQuery`, validates both sides with
the same outer scope rules as subqueries, checks column count, computes compatible
output column types, and validates final `ORDER BY` against output aliases. The
renderer can then treat set operations mechanically: render each operand as a
query body, render the set operator, then render final order/limit/offset.

### Sources Are Metadata Plus Validated SQL Shape

`Dataset` remains the metadata wrapper for anything addressable in `FROM` or
`JOIN`: table, view, CTE, raw source, and subquery source. The fields on the
dataset describe what outer filters, sorts, and projections are allowed to use.

Subquery sources validate their `QueryExpr` before rendering and check that the
declared dataset fields match the query output column count and compatible
types. Non-lateral subquery sources validate without outer fields; lateral joins
validate their subquery with the left-side datasets in scope.

### Write Validation Has A Dedicated Scope

Write validation now uses a `WriteScope` over one writable dataset. It still
reuses the generic field resolver internally, but it no longer constructs
throwaway `SelectQuery` values just to validate assignments, filters, or
`RETURNING`.

Validated write structs contain the writable dataset and render-ready resolved
fields, assignments, conflict clauses, filters, and returning items. Write
assignment expressions are validated against the target field type and rendered
with an explicit top-level cast to the target Postgres type. `INSERT`
expressions cannot reference target fields because `VALUES` rows do not have a
current target row; `UPDATE` expressions can reference the row being updated.
Conflict assignments can additionally reference `EXCLUDED.field`. `UPDATE FROM`
and `DELETE USING` add extra datasets to the write scope while assignment
targets stay resolved against the write target.
Validated write models do not keep the original write AST.

### Execution Surface Is Wider Than The Conceptual Model

The user sees query helpers through prelude, so the ergonomics are acceptable,
but internally there are separate execution traits for select, write, and raw
queries. `PgExecutor` now has one low-level method per operation and receives
`StatementCache` from `BuiltQuery`, so raw cache bypass stays explicit without
duplicating cached and uncached executor methods.

Keep this separation unless a later refactor clearly reduces code without
making raw cache bypass or pool/client/transaction support less explicit.

### Row Mapping Has Two Paths

Normal rqb queries map rows using validated `SelectColumn` metadata. Raw queries
map rows from Postgres column type OIDs. Both currently go through a JSON bridge
before serde deserialization.

The implementation keeps those paths separate: typed row mapping is metadata
driven, raw row mapping is OID driven, and shared primitive readers live in the
module root. Both currently go through a JSON bridge before serde
deserialization. Direct row deserialization is a later performance project, not
the first architecture slice.

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
  field::{capabilities, reference, resolved}: field metadata, FieldRef API, resolved fields
  types::{field_type, enum_type, custom}: core types, PG enums, custom type metadata
  ast: QueryExpr, SelectQuery, SetQuery, SearchRequest, Expr, Operator, Aggregate, write ASTs, RawSql
  scope: field and qualifier resolution
  validate: AST -> concrete validated models
  validation::model: render-ready validated structs and enums
  validation::model::ValidatedPredicate: lowered predicate shapes used by renderers
  validation::aggregate: aggregate fields, filters, aliases, and grouping rules
  validation::expr: validated expression tree construction
  validation::sort: validated sort field construction
  validation::value_type: reusable value/type compatibility checks
  validation::value_guard: reusable runtime Value guards
  validation::write: write-specific scope and validated write construction

rqb-postgres
  build: validation + rendering entry points and BuildPostgres traits
  built: rendered query structs and debug SQL display helpers
  error: structured Core/Postgres/runtime error surface
  type_sql::{casts, selection, names}: bind casts, selection repr, type identifiers
  render: validated models -> BuiltQuery
  render::expr: logical expression dispatch
  render::predicate::{comparison, text, collection, target}: concrete predicate SQL
  render::params: Value -> SQL placeholder and cast shape
  params: Value -> ToSql-owned params
  row_map::{typed, raw}: metadata-driven and OID-driven Row -> serde bridge
  row_map::values: shared Row value readers and feature-gated conversions
  executor::driver: PgExecutor implementations for driver/client types and Page
  executor::query: shared low-level query/fetch helpers
  executor::{select, write, raw}: user-facing execution traits
  pool::db: pooled connection construction and transaction entry points
  pool::transaction: BeginBuilder, Tx, Savepoint, rollback-on-drop
  pool::executor: PgExecutor impls for pooled Db, Tx, and Savepoint

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

2. Add a write-specific scope. Done.
   Write validation uses `WriteScope` over a single writable dataset. The scope
   delegates to the generic field resolver, but no longer builds throwaway
   select queries for write validation.

3. Centralize type behavior. Done.
   Type-family and array element metadata live on `FieldType`; custom
   representation helpers live on `ValueRepr`, `SelectRepr`, and `TypeSpec`;
   Postgres cast, selection, type-name, and array-cast behavior lives in
   `rqb-postgres/src/type_sql`; runtime value-shape guards live in validation.

4. Categorize and lower operators. Done.
   Keep `Operator` as the JSON/user-facing enum, but route validation and
   lowering through explicit operator categories. Rendering consumes concrete
   `ValidatedPredicate` shapes and no longer reinterprets user-facing operators.

5. Clean the facade API surface. Done.
   `rqb-core` must expose validated models for `rqb-postgres`, but the `rqb`
   facade uses an explicit ergonomic export list, so internal `Validated*`
   types stay in `rqb_core` instead of appearing under `rqb::*`.

6. Split validated model definitions. Done.
   Render-ready validated structs and enums live in `validation/model.rs`.
   `validation/mod.rs` now wires validation modules together and re-exports the
   validated model for `rqb-postgres`.

7. Revisit execution traits. Done.
   Select, write, and raw user-facing traits stay separate because their
   semantics differ, but the lower `PgExecutor` contract now carries
   `StatementCache` explicitly instead of exposing duplicate cached/uncached
   methods.

8. Split CLI internals. Done.
   Catalog introspection, type mapping, code rendering, identifier hygiene, and
   shared schema models now live in separate modules.

## Non-Goals For This Refactor

- Do not introduce a Diesel-like compile-time type system.
- Do not add `Box<dyn ToSql>` to normal query code.
- Do not make rendering validate user-facing semantics again.
- Do not hide raw SQL safety rules behind convenience APIs.
- Do not preserve awkward API solely for compatibility before public release.
