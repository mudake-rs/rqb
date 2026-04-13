use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use pretty_assertions::assert_eq;

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

    let orders = golden_schema::orders::dataset();
    assert_eq!(orders.source_name(), "orders");
    assert!(orders.fields.iter().any(|field| field.api_name == "status"));

    let order_search = golden_schema::order_search_view::dataset();
    assert_eq!(order_search.source_name(), "order_search_view");
    assert!(
        order_search
            .fields
            .iter()
            .any(|field| field.api_name == "totalCents")
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
    assert!(!generated.contains("pub mod app_users"));
    assert!(!generated.contains("pub mod order_search_view"));

    fs::remove_file(output_path).expect("failed to remove generated filtered schema output");
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
