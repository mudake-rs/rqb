use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;

mod codegen;
mod ident;
mod introspect;
mod model;
mod type_map;

use codegen::render;
use introspect::introspect;

#[derive(Parser)]
#[command(name = "rqb")]
#[command(version)]
#[command(about = "Schema introspection and code generation for rqb")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Generate an rqb::schema! module from a Postgres schema")]
    Generate {
        #[arg(long, env = "DATABASE_URL", help = "Postgres connection URL")]
        database_url: String,
        #[arg(long, default_value = "public", help = "Postgres schema to introspect")]
        schema: String,
        #[arg(long, help = "Limit generation to a table or view; may be repeated")]
        table: Vec<String>,
        #[arg(long, help = "Output Rust file path")]
        out: PathBuf,
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
            out,
        } => generate(&database_url, &schema, &table, out).await,
    }
}

async fn generate(
    database_url: &str,
    schema: &str,
    only_tables: &[String],
    out: PathBuf,
) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("failed to connect to Postgres")?;

    let mut relations = introspect(&pool, schema, only_tables).await?;
    relations.sort_by(|a, b| a.name.cmp(&b.name));

    let code = render(&relations)?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&out, code).with_context(|| format!("failed to write {}", out.display()))?;
    Ok(())
}
