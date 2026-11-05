use edge_media_probe::control_plane::{create_control_plane_router, AppState};
use edge_media_probe::observability::spawn_runtime_metrics_collector;
use metrics_exporter_prometheus::PrometheusBuilder;
use sqlx::SqlitePool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Inicjalizacja subskrybenta metryk Prometheusa
    let recorder = PrometheusBuilder::new().install_recorder()?;
    spawn_runtime_metrics_collector();

    // Baza danych SQLite w pamięci (lub plikowa)
    let pool = SqlitePool::connect("sqlite::memory:").await?;
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

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 [Control Plane] Serwer nasłuchuje na http://{addr}");
    println!("   • Endpoint KMS:        http://{addr}/v1/keys/:ssrc");
    println!("   • Webhook alertów:     http://{addr}/v1/alerts");
    println!("   • Metryki Prometheusa: http://{addr}/metrics");

    axum::serve(listener, app).await?;
    Ok(())
}

