use std::sync::OnceLock;

use divan::{Bencher, black_box};
use rqb_core::{Dataset, ElemType, Field, FieldType, JsonPathPolicy, SelectQuery, select};
use rqb_postgres::ExecutePostgres;
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio_postgres::{Client, NoTls};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const ID: Field = Field::new("id", FieldType::BigInt);
const EMAIL: Field = Field::new("email", FieldType::Text);
const STATUS: Field = Field::new("status", FieldType::Text);
const TAGS: Field = Field::new("tags", FieldType::Array(ElemType::Text)).sortable(false);
const METADATA: Field = Field::new("metadata", FieldType::Jsonb)
    .sortable(false)
    .json_paths(JsonPathPolicy::Dynamic);
const TOTAL: Field = Field::new("total", FieldType::Numeric);
const ACTIVE: Field = Field::new("active", FieldType::Bool);

const FIELDS: &[Field] = &[ID, EMAIL, STATUS, TAGS, METADATA, TOTAL, ACTIVE];

fn main() {
    divan::main();
}

#[derive(Debug, Deserialize)]
struct BenchRow {
    id: i64,
    email: String,
    status: String,
    tags: Vec<String>,
    metadata: BenchMetadata,
    total: String,
    active: bool,
}

#[derive(Debug, Deserialize)]
struct BenchMetadata {
    score: i64,
    gift: bool,
}

fn bench_source() -> Dataset {
    Dataset::raw(
        "\
        SELECT \
            gs::bigint AS id, \
            ('user' || gs || '@example.com')::text AS email, \
            CASE WHEN gs % 2 = 0 THEN 'paid' ELSE 'draft' END::text AS status, \
            ARRAY['vip', 'gift']::text[] AS tags, \
            jsonb_build_object('score', gs, 'gift', gs % 2 = 0) AS metadata, \
            (gs * 100)::numeric AS total, \
            (gs % 2 = 0) AS active \
        FROM generate_series(1, 100) AS gs",
        "bench_rows",
    )
    .static_fields(FIELDS)
    .max_limit(200)
}

fn bench_query() -> SelectQuery {
    select(bench_source())
        .fields(FIELDS.iter().copied())
        .limit(100)
        .build()
}

#[divan::bench(
    ignore = database_url().is_none(),
    sample_count = 20,
    sample_size = 1
)]
fn rqb_fetch_all_100_rows(bencher: Bencher) {
    let rt = runtime();
    let client = rt.block_on(connect_client());

    bencher.bench_local(|| {
        rt.block_on(async {
            let rows = bench_query()
                .fetch_all(&client)
                .await
                .expect("fetch_all should run");
            black_box(rows.len())
        })
    });
}

#[divan::bench(
    ignore = database_url().is_none(),
    sample_count = 20,
    sample_size = 1
)]
fn rqb_fetch_all_as_100_rows(bencher: Bencher) {
    let rt = runtime();
    let client = rt.block_on(connect_client());

    bencher.bench_local(|| {
        rt.block_on(async {
            let rows = bench_query()
                .fetch_all_as::<BenchRow>(&client)
                .await
                .expect("fetch_all_as should run");
            black_box(consume_rows(&rows))
        })
    });
}

fn consume_rows(rows: &[BenchRow]) -> usize {
    rows.iter()
        .map(|row| {
            row.email.len()
                + row.status.len()
                + row.tags.iter().map(String::len).sum::<usize>()
                + row.total.len()
                + usize::from(row.active)
                + row.id.unsigned_abs() as usize
                + row.metadata.score.unsigned_abs() as usize
                + usize::from(row.metadata.gift)
        })
        .sum()
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("tokio runtime should start"))
}

async fn connect_client() -> Client {
    let (client, connection) = tokio_postgres::connect(
        &database_url().expect("database URL checked by divan ignore"),
        NoTls,
    )
    .await
    .expect("Postgres connection should open");
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("Postgres benchmark connection error: {error}");
        }
    });
    client
}

fn database_url() -> Option<String> {
    std::env::var("RQB_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}
