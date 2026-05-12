use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;

mod codegen;
mod ident;
mod introspect;
mod model;
mod type_map;

use codegen::render;
use introspect::introspect;

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
            stdout,
            check,
            no_rustfmt,
            out,
        } => {
            generate(
                &database_url,
                &schema,
                &table,
                GenerateOutput {
                    out,
                    stdout,
                    check,
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
    no_rustfmt: bool,
}

async fn generate(
    database_url: &str,
    schema: &str,
    only_tables: &[String],
    output: GenerateOutput,
) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    let mut relations = introspect(&pool, schema, only_tables).await?;
    if relations.is_empty() {
        bail!("no tables, views, or materialized views found in schema `{schema}`");
    }
    relations.sort_by(|a, b| a.name.cmp(&b.name));

    let code = format_generated_code(&render(&relations)?, output.no_rustfmt)?;
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
        relations.len(),
        out.display()
    );
    Ok(())
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
    use clap::Parser;

    use super::Cli;

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
}
