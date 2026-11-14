use std::collections::HashMap;
use std::sync::{Arc, Mutex};
//
use edge_probe::control_plane::{create_router, init_db, spawn_alert_writer, AppState};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = init_db("sqlite://control_plane.db?mode=rwc").await?;
    let (alerts_tx, alerts_rx) = mpsc::channel(64);
    spawn_alert_writer(db, alerts_rx);

    let state = Arc::new(AppState {
        streams: Mutex::new(HashMap::new()),
        alerts_tx,
    });

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("control-plane nasłuchuje na http://127.0.0.1:8080");

    axum::serve(listener, create_router(state))
        .with_graceful_shutdown(async {
            let _ = signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
