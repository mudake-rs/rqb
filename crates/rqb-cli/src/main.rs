use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio_postgres::NoTls;

mod codegen;
mod ident;
mod introspect;
mod model;
mod type_map;

use codegen::render;
use introspect::{introspect, introspect_domains, introspect_enums};

#[derive(Parser)]
#[command(name = "rqb")]
#[command(about = "Schema introspection and code generation for rqb")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Generate {
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,
        #[arg(long, default_value = "public")]
        schema: String,
        #[arg(long)]
        table: Vec<String>,
        #[arg(long)]
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
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .context("failed to connect to Postgres")?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("postgres connection error: {error}");
        }
    });

    let enums = introspect_enums(&client, schema).await?;
    let domains = introspect_domains(&client, schema).await?;
    let mut relations = introspect(&client, schema, only_tables, &enums, &domains).await?;
    relations.sort_by(|a, b| a.name.cmp(&b.name));

    let code = render(&relations, &enums, &domains)?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&out, code).with_context(|| format!("failed to write {}", out.display()))?;
    Ok(())
}
