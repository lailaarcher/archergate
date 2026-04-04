//! Archergate License Server — standalone REST API for license validation.
//!
//! Usage:
//!   archergate-license-server serve --port 3100 --db ./licenses.db
//!   archergate-license-server create-key --email dev@example.com
//!   archergate-license-server create-license --plugin com.dev.synth --max-machines 3

use archergate_license_server::db;
use archergate_license_server::handlers;

use std::path::PathBuf;
use std::sync::Arc;


use clap::{Parser, Subcommand};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Parser)]
#[command(name = "archergate-license-server")]
#[command(about = "License validation server for indie software developers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the license server.
    Serve {
        /// Port to listen on.
        #[arg(short, long, default_value = "3100")]
        port: u16,

        /// Path to the SQLite database file.
        #[arg(short, long, default_value = "./archergate-licenses.db")]
        db: PathBuf,
    },

    /// Create a new developer API key.
    CreateKey {
        /// Developer email address.
        #[arg(short, long)]
        email: String,

        /// Path to the SQLite database file.
        #[arg(short, long, default_value = "./archergate-licenses.db")]
        db: PathBuf,
    },

    /// Create a new license key for a plugin.
    CreateLicense {
        /// Plugin identifier (e.g. com.yourname.synth).
        #[arg(short, long)]
        plugin: String,

        /// Customer email address.
        #[arg(short, long)]
        email: Option<String>,

        /// Maximum number of machines (default: 3).
        #[arg(short, long, default_value = "3")]
        max_machines: i32,

        /// Expiration date (ISO 8601, e.g. 2025-12-31T00:00:00Z). Omit for perpetual.
        #[arg(long)]
        expires: Option<String>,

        /// Developer API key ID to associate this license with.
        #[arg(short, long)]
        api_key_id: String,

        /// Path to the SQLite database file.
        #[arg(short, long, default_value = "./archergate-licenses.db")]
        db: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "archergate_license_server=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, db: db_path } => {
            let database = db::Db::open(&db_path).expect("Failed to open database");
            let state: handlers::AppState = Arc::new(database);

            let app = handlers::build_router(state)
                .layer(CorsLayer::permissive())
                .layer(TraceLayer::new_for_http())
                .into_make_service();

            let addr = format!("0.0.0.0:{port}");
            tracing::info!("Archergate License Server listening on {addr}");
            tracing::info!("Database: {}", db_path.display());

            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
            // axum::serve requires IntoMakeService, provided by into_make_service() above
        }

        Commands::CreateKey { email, db: db_path } => {
            let database = db::Db::open(&db_path).expect("Failed to open database");
            let (raw_key, record) = database.create_api_key(&email).unwrap();
            println!("API Key created:");
            println!("  Key:   {raw_key}");
            println!("  ID:    {}", record.id);
            println!("  Email: {email}");
            println!();
            println!("Save this key — it cannot be retrieved later.");
        }

        Commands::CreateLicense {
            plugin,
            email,
            max_machines,
            expires,
            api_key_id,
            db: db_path,
        } => {
            let database = db::Db::open(&db_path).expect("Failed to open database");
            let license = database
                .create_license(
                    &plugin,
                    email.as_deref(),
                    expires.as_deref(),
                    max_machines,
                    &api_key_id,
                )
                .unwrap();
            println!("License created:");
            println!("  Key:          {}", license.license_key);
            println!("  Plugin:       {plugin}");
            println!("  Max machines: {max_machines}");
            if let Some(exp) = expires {
                println!("  Expires:      {exp}");
            } else {
                println!("  Expires:      never (perpetual)");
            }
        }
    }
}
