use clap::Parser;
use edge_media_probe::control_plane::{create_control_plane_router, AppState};
use edge_media_probe::observability::{init_tracing, spawn_runtime_metrics_collector};
use metrics_exporter_prometheus::PrometheusBuilder;
use sqlx::SqlitePool;
use std::net::SocketAddr;
use std::sync::Arc;

/// Serwer sterujący i KMS (Control Plane) dla sond brzegowych
#[derive(Parser, Debug)]
#[command(name = "control_plane", version, about = "Serwer sterujący i KMS (Control Plane) dla sond brzegowych")]
struct Args {
    /// Adres nasłuchu serwera HTTP / KMS / metryk
    #[arg(short, long, default_value = "0.0.0.0:3000")]
    listen: SocketAddr,

    /// URL połączenia z bazą danych SQLite
    #[arg(long, default_value = "sqlite::memory:")]
    database_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Inicjalizacja nieblokującego potoku telemetrii (tracing + opcjonalnie Tokio Console)
    let _guard = init_tracing("control_plane");

    // Inicjalizacja subskrybenta metryk Prometheusa
    let recorder = PrometheusBuilder::new().install_recorder()?;
    spawn_runtime_metrics_collector();

    // Baza danych SQLite w pamięci (lub plikowa)
    let pool = SqlitePool::connect(&args.database_url).await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS probe_alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ssrc INTEGER NOT NULL,
            fraction_lost REAL NOT NULL,
            reason TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    )
    .execute(&pool)
    .await?;

    let state = Arc::new(AppState { db: pool });
    let app = create_control_plane_router(state)
        .route(
            "/metrics",
            axum::routing::get(move || async move { recorder.render() }),
        );

    let addr = args.listen;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 [Control Plane] Serwer nasłuchuje na http://{addr}");
    println!("   • Endpoint KMS:        http://{addr}/v1/keys/:ssrc");
    println!("   • Webhook alertów:     http://{addr}/v1/alerts");
    println!("   • Metryki Prometheusa: http://{addr}/metrics");

    axum::serve(listener, app).await?;
    Ok(())
}

