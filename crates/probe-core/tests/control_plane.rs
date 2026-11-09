use probe_core::control_plane::{
    create_control_plane_router, peek_control_protocol, AppState, ControlProtocol,
};
use probe_core::kms_client::{KmsClient, SlaAlertPayload};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_end_to_end_kms_client_with_axum_server() -> anyhow::Result<()> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    sqlx::query(
        "CREATE TABLE probe_alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ssrc INTEGER NOT NULL,
            fraction_lost REAL NOT NULL,
            reason TEXT NOT NULL
        );"
    )
    .execute(&pool)
    .await?;

    let state = Arc::new(AppState { db: pool.clone() });
    let app = create_control_plane_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let server_addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = KmsClient::new(format!("http://{server_addr}"))?;

    let key_material = client.fetch_srtp_keys(0x1234_5678).await?;
    assert_eq!(key_material.ssrc, 0x1234_5678);
    assert_eq!(key_material.master_key.len(), 16);
    assert_eq!(key_material.master_salt.len(), 14);

    let alert = SlaAlertPayload {
        probe_id: "probe-rpi-01".to_string(),
        ssrc: 0x1234_5678,
        fraction_lost: 0.08,
        reason: "Pakiet NACK: utrata klatek kluczowych".to_string(),
    };
    client.send_sla_alert(&alert).await?;

    let row: (i64, f64, String) = sqlx::query_as("SELECT ssrc, fraction_lost, reason FROM probe_alerts")
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.0, 0x1234_5678);
    assert!((row.1 - 0.08).abs() < 1e-4);
    assert_eq!(row.2, "Pakiet NACK: utrata klatek kluczowych");

    Ok(())
}

#[tokio::test]
async fn test_peek_control_protocol_preserves_stream_bytes() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client.write_all(b"GET /v1/keys/0xCAFEBABE HTTP/1.1\r\n").await.unwrap();
    });

    let (mut server_stream, _) = listener.accept().await?;
    let proto = peek_control_protocol(&server_stream).await?;
    assert_eq!(proto, ControlProtocol::Http);

    // Weryfikacja, że bajty "GET " pozostały w buforze gniazda dla parsera HTTP:
    let mut prefix = [0u8; 4];
    server_stream.read_exact(&mut prefix).await?;
    assert_eq!(&prefix, b"GET ");

    Ok(())
}

