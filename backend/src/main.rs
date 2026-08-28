mod auth;
mod cli;
mod config;
mod error;
mod routes;
mod storage;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::ServerConfig;
use crate::error::ServerError;
use crate::routes::AppCtx;
use crate::storage::Storage;

/// Granit cave backup server.
#[derive(Parser)]
#[command(name = "granit-server", version, about)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server (default).
    Serve,
    /// Create a new API key and print its token once.
    CreateKey {
        /// Human-readable key name, e.g. the machine it belongs to.
        #[arg(long)]
        name: String,
    },
    /// List all API keys.
    ListKeys,
    /// Revoke an API key by id.
    RevokeKey {
        #[arg(long)]
        id: Uuid,
    },
}

async fn connect_pool() -> Result<PgPool, ServerError> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| ServerError::Config("DATABASE_URL is not set".to_string()))?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|err| ServerError::Config(format!("migration failed: {err}")))?;
    Ok(pool)
}

async fn serve() -> Result<(), ServerError> {
    let config = ServerConfig::from_env()?;
    let pool = connect_pool().await?;

    let storage = Arc::new(Storage::new(&config.s3));
    storage.ensure_bucket().await?;

    let router = routes::router(AppCtx { db: pool, storage });
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .map_err(|err| ServerError::Config(format!("cannot bind {}: {err}", config.bind_addr)))?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, router)
        .await
        .map_err(|err| ServerError::Config(format!("server failed: {err}")))?;
    Ok(())
}

async fn run(args: Args) -> Result<(), ServerError> {
    match args.command.unwrap_or(Command::Serve) {
        Command::Serve => serve().await,
        Command::CreateKey { name } => cli::create_key(&connect_pool().await?, &name).await,
        Command::ListKeys => cli::list_keys(&connect_pool().await?).await,
        Command::RevokeKey { id } => cli::revoke_key(&connect_pool().await?, id).await,
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "granit_server=info,tower_http=info".into()),
        )
        .init();

    if let Err(err) = run(Args::parse()).await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
