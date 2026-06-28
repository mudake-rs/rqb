use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;

mod codegen;
mod config;
mod ident;
mod introspect;
mod model;
mod type_map;

use codegen::render;
use config::GeneratorConfig;
use introspect::introspect;
use model::{ColumnType, Relation};

#[derive(Debug, Parser)]
#[command(name = "rqb")]
#[command(version)]
#[command(about = "Schema introspection and code generation for rqb")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Generate an rqb::schema! module from a Postgres schema")]
    Generate {
        #[arg(long, env = "DATABASE_URL", help = "Postgres connection URL")]
        database_url: String,
        #[arg(long, default_value = "public", help = "Postgres schema to introspect")]
        schema: String,
        #[arg(
            long,
            help = "Limit generation to a table, view, or materialized view; may be repeated"
        )]
        table: Vec<String>,
        #[arg(long, help = "Path to rqb-cli generator config TOML")]
        config: Option<PathBuf>,
        #[arg(
            long,
            conflicts_with_all = ["check", "out"],
            help = "Print generated code to stdout instead of writing --out"
        )]
        stdout: bool,
        #[arg(
            long,
            requires = "out",
            help = "Exit with an error if --out differs from generated code"
        )]
        check: bool,
        #[arg(long, help = "Print a schema generation report to stderr")]
        report: bool,
        #[arg(
            long,
            help = "Fail when raw-only columns are not listed in [raw_only].allow"
        )]
        deny_raw_only: bool,
        #[arg(
            long,
            help = "Fail when --config has type_map entries unused by the selected schema"
        )]
        deny_unused_type_map: bool,
        #[arg(long, help = "Skip rustfmt on generated output")]
        no_rustfmt: bool,
        #[arg(
            long,
            required_unless_present = "stdout",
            help = "Output Rust file path"
        )]
        out: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            database_url,
            schema,
            table,
            config,
            stdout,
            check,
            report,
            deny_raw_only,
            deny_unused_type_map,
            no_rustfmt,
            out,
        } => {
            generate(
                &database_url,
                &schema,
                &table,
                config.as_deref(),
                GenerateOutput {
                    out,
                    stdout,
                    check,
                    report,
                    deny_raw_only,
                    deny_unused_type_map,
                    no_rustfmt,
                },
            )
            .await
        }
    }
}

struct GenerateOutput {
    out: Option<PathBuf>,
    stdout: bool,
    check: bool,
    report: bool,
    deny_raw_only: bool,
    deny_unused_type_map: bool,
    no_rustfmt: bool,
}

async fn generate(
    database_url: &str,
    schema: &str,
    only_tables: &[String],
    config_path: Option<&std::path::Path>,
    output: GenerateOutput,
) -> Result<()> {
    let config = GeneratorConfig::load(config_path)?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    let introspection = introspect(&pool, schema, only_tables, &config.type_map).await?;
    let mut schema_model = introspection.schema;
    if schema_model.relations.is_empty() {
        bail!("no tables, views, or materialized views found in schema `{schema}`");
    }
    schema_model.relations.sort_by(|a, b| a.name.cmp(&b.name));
    let raw_only = raw_only_columns(&schema_model.relations);
    let unused_type_mappings =
        unused_type_mappings(&config.type_map, &introspection.used_type_mappings);
    if output.report {
        report_generation(&schema_model.relations, &raw_only, &unused_type_mappings);
    } else {
        report_warnings(&raw_only, &unused_type_mappings);
    }

    if output.deny_raw_only {
        let denied = unallowed_raw_only_columns(&raw_only, &config.raw_only.allow);
        if !denied.is_empty() {
            let items = denied
                .iter()
                .map(RawOnlyColumn::display)
                .collect::<Vec<_>>();
            bail!(
                "raw-only columns are not allowed: {}",
                summarize_items(&items)
            );
        }
    }
    if output.deny_unused_type_map && !unused_type_mappings.is_empty() {
        bail!(
            "unused type_map entries: {}",
            summarize_items(&unused_type_mappings)
        );
    }

    let code = format_generated_code(&render(&schema_model)?, output.no_rustfmt)?;
    if output.stdout {
        print!("{code}");
        return Ok(());
    }

    let out = output
        .out
        .as_ref()
        .context("--out is required unless --stdout is set")?;

    if output.check {
        let existing =
            fs::read_to_string(out).with_context(|| format!("failed to read {}", out.display()))?;
        let existing = format_generated_code(&existing, output.no_rustfmt)?;
        if existing != code {
            bail!(
                "schema drift detected: {} differs from generated output",
                out.display()
            );
        }
        println!("rqb-cli: schema is up to date: {}", out.display());
        return Ok(());
    }

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(out, code).with_context(|| format!("failed to write {}", out.display()))?;
    println!(
        "rqb-cli: generated {} relation(s) to {}",
        schema_model.relations.len(),
        out.display()
    );
    Ok(())
}

fn report_warnings(raw_only: &[RawOnlyColumn], unused_type_mappings: &[String]) {
    if !raw_only.is_empty() {
        let items = raw_only
            .iter()
            .map(RawOnlyColumn::display)
            .collect::<Vec<_>>();
        eprintln!(
            "rqb-cli: {} raw-only column(s): {}",
            raw_only.len(),
            summarize_items(&items)
        );
    }
    if !unused_type_mappings.is_empty() {
        eprintln!(
            "rqb-cli: {} unused type_map {}: {}",
            unused_type_mappings.len(),
            plural(unused_type_mappings.len(), "entry", "entries"),
            summarize_items(unused_type_mappings)
        );
    }
}

fn report_generation(
    relations: &[Relation],
    raw_only: &[RawOnlyColumn],
    unused_type_mappings: &[String],
) {
    let stats = schema_stats(relations);
    eprintln!("rqb-cli report:");
    eprintln!("  relations: {}", relations.len());
    eprintln!(
        "  columns: {} known, {} custom, {} enum, {} raw-only",
        stats.known_columns, stats.custom_columns, stats.enum_columns, stats.raw_only_columns
    );
    eprintln!("  constraints: {}", stats.constraints);
    if raw_only.is_empty() {
        eprintln!("  raw-only: none");
    } else {
        eprintln!("  raw-only:");
        for column in raw_only {
            eprintln!("    - {}", column.display());
        }
    }
    if unused_type_mappings.is_empty() {
        eprintln!("  unused type_map: none");
    } else {
        eprintln!("  unused type_map:");
        for key in unused_type_mappings {
            eprintln!("    - {key}");
        }
    }
}

#[derive(Default)]
struct SchemaStats {
    known_columns: usize,
    custom_columns: usize,
    enum_columns: usize,
    raw_only_columns: usize,
    constraints: usize,
}

fn schema_stats(relations: &[Relation]) -> SchemaStats {
    let mut stats = SchemaStats::default();
    for relation in relations {
        stats.constraints += relation.constraints.len();
        for column in &relation.columns {
            match &column.ty {
                ColumnType::Known(_) => stats.known_columns += 1,
                ColumnType::Custom { .. } => stats.custom_columns += 1,
                ColumnType::PgEnum { .. } => stats.enum_columns += 1,
                ColumnType::RawOnly { .. } => stats.raw_only_columns += 1,
            }
        }
    }
    stats
}

fn summarize_items(items: &[String]) -> String {
    let shown = items
        .iter()
        .take(12)
        .map(String::as_str)
        .collect::<Vec<_>>();
    let suffix = items
        .len()
        .checked_sub(shown.len())
        .filter(|remaining| *remaining > 0)
        .map_or(String::new(), |remaining| format!("; and {remaining} more"));
    format!("{}{}", shown.join(", "), suffix)
}

fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawOnlyColumn {
    key: String,
    pg: String,
}

impl RawOnlyColumn {
    fn display(&self) -> String {
        format!("{} ({})", self.key, self.pg)
    }
}

fn unallowed_raw_only_columns(
    raw_only: &[RawOnlyColumn],
    allow: &BTreeSet<String>,
) -> Vec<RawOnlyColumn> {
    raw_only
        .iter()
        .filter(|column| !allow.contains(&column.key))
        .cloned()
        .collect()
}

fn unused_type_mappings(
    configured: &config::TypeMappings,
    used: &BTreeSet<(String, String)>,
) -> Vec<String> {
    configured
        .keys()
        .filter(|key| !used.contains(*key))
        .map(type_mapping_key)
        .collect()
}

fn type_mapping_key(key: &(String, String)) -> String {
    format!("{}.{}", key.0, key.1)
}

fn raw_only_columns(relations: &[Relation]) -> Vec<RawOnlyColumn> {
    relations
        .iter()
        .flat_map(|relation| {
            relation.columns.iter().filter_map(move |column| {
                if let ColumnType::RawOnly { pg } = &column.ty {
                    Some(RawOnlyColumn {
                        key: format!("{}.{}.{}", relation.schema, relation.name, column.name),
                        pg: pg.clone(),
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}

fn format_generated_code(code: &str, no_rustfmt: bool) -> Result<String> {
    if no_rustfmt {
        return Ok(code.to_owned());
    }

    let mut child = ProcessCommand::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("rustfmt not found on PATH; pass --no-rustfmt to skip formatting")?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .context("failed to open rustfmt stdin")?;
        stdin
            .write_all(code.as_bytes())
            .context("failed to write generated code to rustfmt")?;
    }

    let output = child
        .wait_with_output()
        .context("failed to wait for rustfmt")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("rustfmt failed: {}", stderr.trim());
    }
    String::from_utf8(output.stdout).context("rustfmt emitted non-UTF-8 output")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use clap::Parser;

    use super::{
        Cli, RawOnlyColumn, raw_only_columns, schema_stats, unallowed_raw_only_columns,
        unused_type_mappings,
    };
    use crate::config::TypeMapping;
    use crate::model::{
        Column, ColumnType, FieldJson, FieldOps, GeneratedKind, KnownType, Relation, RelationKind,
        UniqueConstraint,
    };

    #[test]
    fn stdout_and_check_conflict() {
        let err = Cli::try_parse_from([
            "rqb",
            "generate",
            "--database-url",
            "postgres://localhost/db",
            "--stdout",
            "--check",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn stdout_and_out_conflict() {
        let err = Cli::try_parse_from([
            "rqb",
            "generate",
            "--database-url",
            "postgres://localhost/db",
            "--stdout",
            "--out",
            "schema.rs",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn out_required_unless_stdout_is_set() {
        let err = Cli::try_parse_from([
            "rqb",
            "generate",
            "--database-url",
            "postgres://localhost/db",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

        Cli::try_parse_from([
            "rqb",
            "generate",
            "--database-url",
            "postgres://localhost/db",
            "--stdout",
        ])
        .unwrap();
    }

    #[test]
    fn check_requires_out() {
        let err = Cli::try_parse_from([
            "rqb",
            "generate",
            "--database-url",
            "postgres://localhost/db",
            "--check",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn generate_accepts_config_path_and_ci_policy_flags() {
        Cli::try_parse_from([
            "rqb",
            "generate",
            "--database-url",
            "postgres://localhost/db",
            "--config",
            "rqb.toml",
            "--report",
            "--deny-raw-only",
            "--deny-unused-type-map",
            "--stdout",
        ])
        .unwrap();
    }

    #[test]
    fn raw_only_column_report_lists_extension_columns() {
        let columns = raw_only_columns(&[Relation {
            schema: "public".to_owned(),
            name: "documents".to_owned(),
            kind: RelationKind::Table,
            columns: vec![
                Column {
                    name: "id".to_owned(),
                    const_name: "ID".to_owned(),
                    ty: ColumnType::Known(KnownType::Uuid),
                    nullable: false,
                    generated: GeneratedKind::None,
                },
                Column {
                    name: "embedding".to_owned(),
                    const_name: "EMBEDDING".to_owned(),
                    ty: ColumnType::RawOnly {
                        pg: "vector(384)".to_owned(),
                    },
                    nullable: false,
                    generated: GeneratedKind::None,
                },
            ],
            constraints: Vec::new(),
        }]);

        assert_eq!(
            columns,
            [RawOnlyColumn {
                key: "public.documents.embedding".to_owned(),
                pg: "vector(384)".to_owned(),
            }]
        );
    }

    #[test]
    fn raw_only_allowlist_filters_denied_columns() {
        let raw_only = vec![
            RawOnlyColumn {
                key: "public.documents.embedding".to_owned(),
                pg: "vector(384)".to_owned(),
            },
            RawOnlyColumn {
                key: "public.documents.search".to_owned(),
                pg: "tsvector".to_owned(),
            },
        ];
        let allow = BTreeSet::from(["public.documents.embedding".to_owned()]);

        assert_eq!(
            unallowed_raw_only_columns(&raw_only, &allow),
            [RawOnlyColumn {
                key: "public.documents.search".to_owned(),
                pg: "tsvector".to_owned(),
            }]
        );
    }

    #[test]
    fn unused_type_mappings_reports_config_entries_not_seen() {
        let configured = BTreeMap::from([
            (
                ("bitcoin".to_owned(), "uint256".to_owned()),
                TypeMapping {
                    rust: "crate::types::PgU256".to_owned(),
                    ops: FieldOps::Ordered,
                    json: Some(FieldJson::Text),
                    array: true,
                },
            ),
            (
                ("public".to_owned(), "vector".to_owned()),
                TypeMapping {
                    rust: "pgvector::Vector".to_owned(),
                    ops: FieldOps::None,
                    json: None,
                    array: false,
                },
            ),
        ]);
        let used = BTreeSet::from([("bitcoin".to_owned(), "uint256".to_owned())]);

        assert_eq!(unused_type_mappings(&configured, &used), ["public.vector"]);
    }

    #[test]
    fn schema_stats_counts_generated_surface() {
        let stats = schema_stats(&[Relation {
            schema: "public".to_owned(),
            name: "documents".to_owned(),
            kind: RelationKind::Table,
            columns: vec![
                Column {
                    name: "id".to_owned(),
                    const_name: "ID".to_owned(),
                    ty: ColumnType::Known(KnownType::Uuid),
                    nullable: false,
                    generated: GeneratedKind::None,
                },
                Column {
                    name: "state".to_owned(),
                    const_name: "STATE".to_owned(),
                    ty: ColumnType::PgEnum {
                        schema: "public".to_owned(),
                        name: "doc_state".to_owned(),
                        pg: "doc_state".to_owned(),
                        array: false,
                    },
                    nullable: false,
                    generated: GeneratedKind::None,
                },
                Column {
                    name: "embedding".to_owned(),
                    const_name: "EMBEDDING".to_owned(),
                    ty: ColumnType::Custom {
                        pg: "vector".to_owned(),
                        rust: "pgvector::Vector".to_owned(),
                        array: false,
                        ops: FieldOps::None,
                        json: None,
                    },
                    nullable: false,
                    generated: GeneratedKind::None,
                },
                Column {
                    name: "search".to_owned(),
                    const_name: "SEARCH".to_owned(),
                    ty: ColumnType::RawOnly {
                        pg: "tsvector".to_owned(),
                    },
                    nullable: false,
                    generated: GeneratedKind::None,
                },
            ],
            constraints: vec![UniqueConstraint {
                name: "documents_pkey".to_owned(),
                const_name: "DOCUMENTS_PKEY".to_owned(),
            }],
        }]);

        assert_eq!(stats.known_columns, 1);
        assert_eq!(stats.enum_columns, 1);
        assert_eq!(stats.custom_columns, 1);
        assert_eq!(stats.raw_only_columns, 1);
        assert_eq!(stats.constraints, 1);
    }
}
