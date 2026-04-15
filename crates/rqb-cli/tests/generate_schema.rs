use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use pretty_assertions::assert_eq;
use tokio_postgres::NoTls;

#[path = "golden/public_schema.rs"]
mod golden_schema;

#[test]
fn checked_in_golden_schema_compiles_and_exposes_expected_metadata() {
    assert!(golden_schema::enums::ORDER_STATUS.contains("paid"));
    assert!(golden_schema::enums::USER_STATUS.contains("active"));
    assert_eq!(
        serde_json::to_value(golden_schema::enums::OrderStatus::Paid).unwrap(),
        serde_json::json!("paid")
    );
    assert_eq!(
        serde_json::from_value::<golden_schema::enums::OrderStatus>(serde_json::json!("cancelled"))
            .unwrap(),
        golden_schema::enums::OrderStatus::Cancelled
    );
    assert!(
        serde_json::from_value::<golden_schema::enums::OrderStatus>(serde_json::json!("missing"))
            .is_err()
    );
    assert_eq!(golden_schema::types::UINT_256.name, "uint_256");
    assert_eq!(golden_schema::withdrawals::AMOUNT.ty.as_str(), "uint_256");
    assert!(matches!(
        golden_schema::withdrawals::AMOUNT_HISTORY.ty,
        rqb::prelude::FieldType::Array(rqb::prelude::ElemType::Custom(type_spec))
            if type_spec.name == "uint_256"
    ));

    let orders = golden_schema::orders::dataset();
    assert_eq!(orders.source_name(), "orders");
    assert!(matches!(
        &orders.source,
        rqb::Source::Table {
            schema: Some(schema),
            name,
            ..
        } if schema.as_ref() == "public" && name.as_ref() == "orders"
    ));
    assert!(orders.fields.iter().any(|field| field.api_name == "status"));

    let order_search = golden_schema::order_search_view::dataset();
    assert_eq!(order_search.source_name(), "order_search_view");
    assert!(matches!(
        &order_search.source,
        rqb::Source::View {
            schema: Some(schema),
            name,
            ..
        } if schema.as_ref() == "public" && name.as_ref() == "order_search_view"
    ));
    assert!(
        order_search
            .fields
            .iter()
            .any(|field| field.api_name == "totalCents")
    );

    let pg_types = golden_schema::pg_type_examples::dataset();
    assert!(pg_types.fields.iter().any(|field| {
        field.api_name == "displayName" && field.ty == rqb::prelude::FieldType::Citext
    }));
    assert!(
        pg_types
            .fields
            .iter()
            .any(|field| field.api_name == "payload" && field.ty == rqb::prelude::FieldType::Bytea)
    );
    assert!(
        pg_types
            .fields
            .iter()
            .any(|field| field.api_name == "ipAddr" && field.ty == rqb::prelude::FieldType::Inet)
    );

    let user = golden_schema::app_users::table().alias("u");
    assert_eq!(user.email().display_name(), "u.email");
}

#[test]
fn generate_public_schema_from_live_postgres_matches_golden() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping rqb-cli e2e test; set RQB_TEST_DATABASE_URL");
        return;
    };

    let output_path = temp_schema_path();
    let _ = fs::remove_file(&output_path);

    let output = Command::new(env!("CARGO_BIN_EXE_rqb"))
        .args([
            "generate",
            "--database-url",
            &database_url,
            "--schema",
            "public",
            "--out",
        ])
        .arg(&output_path)
        .output()
        .expect("failed to run rqb generate");

    assert!(
        output.status.success(),
        "rqb generate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rustfmt = Command::new("rustfmt")
        .arg(&output_path)
        .output()
        .expect("failed to run rustfmt for generated schema");

    assert!(
        rustfmt.status.success(),
        "rustfmt failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rustfmt.stdout),
        String::from_utf8_lossy(&rustfmt.stderr)
    );

    let generated =
        fs::read_to_string(&output_path).expect("failed to read generated schema output");
    let expected = include_str!("golden/public_schema.rs");
    assert_eq!(generated, expected);

    fs::remove_file(output_path).expect("failed to remove generated schema output");
}

#[test]
fn generate_public_schema_can_filter_tables() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping rqb-cli table filter e2e test; set RQB_TEST_DATABASE_URL");
        return;
    };

    let output_path = temp_schema_path();
    let _ = fs::remove_file(&output_path);

    let output = Command::new(env!("CARGO_BIN_EXE_rqb"))
        .args([
            "generate",
            "--database-url",
            &database_url,
            "--schema",
            "public",
            "--table",
            "orders",
            "--table",
            "order_items",
            "--out",
        ])
        .arg(&output_path)
        .output()
        .expect("failed to run rqb generate with table filters");

    assert!(
        output.status.success(),
        "rqb generate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string(&output_path).expect("failed to read generated filtered schema output");
    assert!(generated.contains("pub mod orders"));
    assert!(generated.contains("pub mod order_items"));
    assert!(generated.contains("pub const ORDER_STATUS"));
    assert!(!generated.contains("USER_STATUS"));
    assert!(!generated.contains("UINT_256"));
    assert!(!generated.contains("pub mod app_users"));
    assert!(!generated.contains("pub mod order_search_view"));

    fs::remove_file(output_path).expect("failed to remove generated filtered schema output");
}

#[tokio::test]
async fn generate_filtered_schema_ignores_unused_unsupported_domains() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping rqb-cli unused domain e2e test; set RQB_TEST_DATABASE_URL");
        return;
    };

    let schema = temp_schema_name("unused_domain");
    let output_path = temp_schema_path();
    let _ = fs::remove_file(&output_path);
    drop_schema(&database_url, &schema).await;
    create_unused_domain_schema(&database_url, &schema).await;

    let output = Command::new(env!("CARGO_BIN_EXE_rqb"))
        .args([
            "generate",
            "--database-url",
            &database_url,
            "--schema",
            &schema,
            "--table",
            "ok_table",
            "--out",
        ])
        .arg(&output_path)
        .output()
        .expect("failed to run rqb generate for filtered schema with unused domain");

    drop_schema(&database_url, &schema).await;

    assert!(
        output.status.success(),
        "rqb generate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(&output_path)
        .expect("failed to read generated unused domain schema output");
    assert!(generated.contains("pub mod ok_table"));
    assert!(generated.contains(&format!("\"{schema}\"")));
    assert!(!generated.contains("UNSUPPORTED_DOMAIN"));
    assert!(!generated.contains("UnusedStatus"));

    fs::remove_file(output_path).expect("failed to remove generated unused domain schema output");
}

#[tokio::test]
async fn generate_schema_supports_cross_schema_enums_and_domains() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping rqb-cli cross-schema type e2e test; set RQB_TEST_DATABASE_URL");
        return;
    };

    let table_schema = temp_schema_name("cross_table");
    let type_schema = temp_schema_name("cross_type");
    let output_path = temp_schema_path();
    let _ = fs::remove_file(&output_path);
    drop_schema(&database_url, &table_schema).await;
    drop_schema(&database_url, &type_schema).await;
    create_cross_schema_type_schema(&database_url, &table_schema, &type_schema).await;

    let output = Command::new(env!("CARGO_BIN_EXE_rqb"))
        .args([
            "generate",
            "--database-url",
            &database_url,
            "--schema",
            &table_schema,
            "--table",
            "jobs",
            "--out",
        ])
        .arg(&output_path)
        .output()
        .expect("failed to run rqb generate for cross-schema types");

    drop_schema(&database_url, &table_schema).await;
    drop_schema(&database_url, &type_schema).await;

    assert!(
        output.status.success(),
        "rqb generate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string(&output_path).expect("failed to read generated cross-schema output");
    assert!(generated.contains("pub mod jobs"));
    assert!(generated.contains(&format!("Some(\"{type_schema}\")")));
    assert!(generated.contains(&format!("\"{table_schema}\"")));
    assert!(generated.contains("FieldType::Enum(super::enums::CROSS_STATUS)"));
    assert!(generated.contains("FieldType::Array(ElemType::Enum(super::enums::CROSS_STATUS))"));
    assert!(generated.contains("FieldType::Custom(&"));
    assert!(generated.contains("ElemType::Custom(&"));
    assert!(generated.contains("super::types::CROSS_SCORE"));

    fs::remove_file(output_path).expect("failed to remove generated cross-schema output");
}

#[test]
fn generate_public_schema_rejects_unknown_table_filter() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping rqb-cli unknown table e2e test; set RQB_TEST_DATABASE_URL");
        return;
    };

    let output_path = temp_schema_path();
    let _ = fs::remove_file(&output_path);

    let output = Command::new(env!("CARGO_BIN_EXE_rqb"))
        .args([
            "generate",
            "--database-url",
            &database_url,
            "--schema",
            "public",
            "--table",
            "does_not_exist",
            "--out",
        ])
        .arg(&output_path)
        .output()
        .expect("failed to run rqb generate with missing table filter");

    assert!(
        !output.status.success(),
        "rqb generate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("requested relation(s) not found in schema `public`: does_not_exist"),
        "unexpected stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output_path.exists());
}

fn test_database_url() -> Option<String> {
    std::env::var("RQB_TEST_DATABASE_URL").ok()
}

fn temp_schema_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rqb-cli-public-schema-{}-{nonce}.rs",
        std::process::id()
    ))
}

fn temp_schema_name(prefix: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before UNIX_EPOCH")
        .as_nanos();
    format!("rqb_cli_{prefix}_{}_{nonce}", std::process::id())
}

async fn create_unused_domain_schema(database_url: &str, schema: &str) {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .expect("failed to connect to Postgres for schema setup");
    tokio::spawn(async move {
        let _ = connection.await;
    });

    client
        .batch_execute(&format!(
            r#"
            CREATE SCHEMA "{schema}";
            CREATE DOMAIN "{schema}".unsupported_domain AS macaddr;
            CREATE TYPE "{schema}".unused_status AS ENUM ('unused');
            CREATE TABLE "{schema}".ok_table (
                id uuid PRIMARY KEY,
                name text NOT NULL
            );
            "#
        ))
        .await
        .expect("failed to create unused domain test schema");
}

async fn create_cross_schema_type_schema(
    database_url: &str,
    table_schema: &str,
    type_schema: &str,
) {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .expect("failed to connect to Postgres for cross-schema setup");
    tokio::spawn(async move {
        let _ = connection.await;
    });

    client
        .batch_execute(&format!(
            r#"
            CREATE SCHEMA "{type_schema}";
            CREATE SCHEMA "{table_schema}";
            CREATE TYPE "{type_schema}".cross_status AS ENUM ('ready', 'done');
            CREATE DOMAIN "{type_schema}".cross_score AS numeric;
            CREATE TABLE "{table_schema}".jobs (
                id uuid PRIMARY KEY,
                status "{type_schema}".cross_status NOT NULL,
                status_history "{type_schema}".cross_status[] NOT NULL DEFAULT '{{}}',
                score "{type_schema}".cross_score,
                score_history "{type_schema}".cross_score[]
            );
            "#
        ))
        .await
        .expect("failed to create cross-schema type test schema");
}

async fn drop_schema(database_url: &str, schema: &str) {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .expect("failed to connect to Postgres for schema cleanup");
    tokio::spawn(async move {
        let _ = connection.await;
    });

    client
        .batch_execute(&format!(r#"DROP SCHEMA IF EXISTS "{schema}" CASCADE;"#))
        .await
        .expect("failed to drop unused domain test schema");
}
