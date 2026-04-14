# Benchmarks

rqb has a small benchmark harness for query construction, validation, and
Postgres SQL rendering:

```bash
cargo bench -p rqb-postgres --bench query_build
```

There is also an optional live Postgres harness for `fetch_as` row mapping:

```bash
RQB_TEST_DATABASE_URL=postgres://rqb:rqb@localhost:55432/rqb \
  cargo bench -p rqb-postgres --bench fetch_as --features runtime-tokio-postgres
```

The benchmark uses Divan with allocation profiling enabled, so the output shows
both CPU timing and allocation counts/bytes.

## What It Measures

The benchmark is intentionally focused on application-side overhead:

- building Rust query values
- validating field/operator/type semantics against rqb metadata
- rendering parameterized Postgres SQL
- lowering values into Postgres bind metadata

The `query_build` benchmark does not connect to Postgres and does not measure
network, server planning, execution, row transfer, or row deserialization. The
`fetch_as` benchmark intentionally does connect to Postgres and compares plain
`fetch_all` against typed `fetch_all_as`.

## Competitor Baselines

The benchmark includes nearby Rust baselines:

- `sqlx::QueryBuilder<Postgres>`: manual string builder with bind slots. This is
  the fastest baseline, but it does not provide rqb-style metadata validation,
  field capabilities, JSON search, or typed query-shape semantics.
- SeaQuery: runtime SQL builder. It is closer to rqb structurally, but still
  does less validation than rqb's metadata-driven pipeline.
- Diesel: static typed DSL rendered through `debug_query::<Pg, _>()`. This is
  useful as a static-render reference point, but it is not a dynamic JSON/search
  query builder and its debug SQL is not the same surface as prepared execution.

The results are therefore not a product ranking. They answer a narrower
question: how much CPU and allocation overhead rqb pays for runtime validation
and ergonomic metadata-driven rendering.

## Scenarios

The current harness covers:

- simple SELECT with selected fields, three filters, order, and limit
- the same simple SELECT using generated-style `Field` constants
- nested boolean filter shape
- rqb `SearchRequest` applied to a dataset
- rqb metadata construction
- rqb AST construction vs rendering of a prebuilt query
- raw query placeholder rendering and bind lowering
- live `fetch_all` vs `fetch_all_as` for 100 synthetic Postgres rows

Add a new scenario whenever a hot query path is introduced or a performance
claim needs evidence.

## Current Snapshot

Latest local run on 2026-04-14:

| Scenario | Median | Allocations | Allocated bytes |
| --- | ---: | ---: | ---: |
| `sqlx_query_builder_simple_select` | 714 ns | 4 | 346 B |
| `diesel_debug_simple_select` | 2.69 us | 22 | 598 B |
| `sea_query_simple_select` | 3.22 us | 34 | 5.56 KB |
| `rqb_dataset_metadata` | 119 ns | 0 | 0 B |
| `rqb_simple_select_typed_build_ast` | 771 ns | 5 | 2.56 KB |
| `rqb_simple_select_build_ast` | 1.05 us | 12 | 2.60 KB |
| `rqb_simple_select_typed_render_prebuilt` | 5.13 us | 20 | 6.11 KB |
| `rqb_simple_select_render_prebuilt` | 5.68 us | 20 | 6.11 KB |
| `rqb_simple_select_typed` | 5.79 us | 25 | 8.67 KB |
| `rqb_simple_select` | 6.45 us | 32 | 8.71 KB |
| `rqb_search_request` | 8.25 us | 44 | 8.35 KB |
| `rqb_nested_dynamic_filter` | 10.84 us | 53 | 16.30 KB |
| `rqb_nested_dynamic_render_prebuilt` | 9.25 us | 35 | 11.51 KB |
| `rqb_raw_query_build` | 577 ns | 6 | 725 B |
| `sea_query_nested_dynamic_filter` | 5.82 us | 74 | 13.48 KB |
| `sqlx_query_builder_nested_dynamic_filter` | 851 ns | 4 | 401 B |
| `diesel_debug_nested_static_filter` | 3.16 us | 25 | 759 B |

CPU timings are machine-noisy; allocation counts are the more stable signal.
The generated-style path avoids runtime field-name strings during AST
construction, while the string path remains the right model for JSON/dynamic
request composition.

## Live Row Mapping Snapshot

Latest local run on 2026-04-14 against the test Postgres container:

| Scenario | Median | Allocations | Allocated bytes |
| --- | ---: | ---: | ---: |
| `rqb_fetch_all_100_rows` | 1.23 ms | ~172 | ~49 KB |
| `rqb_fetch_all_as_100_rows` | 1.08 ms | ~1170 | ~114 KB |

The live `fetch_all` and `fetch_all_as` timings include Postgres execution and
row transfer. Use allocation counts for the stable signal in this section, not
absolute wall-clock comparisons across live runs.

On this run, typed `fetch_all_as` adds roughly 1000 allocations per 100 owned
result rows over plain `fetch_all` while preserving `serde::Deserialize`
ergonomics.
