use axum::{routing::get, Json, Router};
use edge_media_probe::kms_client::{KmsClient, KmsKeyResponse, SrtpKeyMaterial};

async fn spawn_mock_kms() -> String {
    let app = Router::new().route(
        "/v1/keys/{ssrc}",
        get(|axum::extract::Path(ssrc_hex): axum::extract::Path<String>| async move {
            let ssrc_val = u32::from_str_radix(ssrc_hex.trim_start_matches("0x"), 16).unwrap_or(0);
            Json(KmsKeyResponse {
                ssrc: ssrc_val,
                master_key_hex: "000102030405060708090a0b0c0d0e0f".to_string(),
                master_salt_hex: "000102030405060708090a0b0c0d".to_string(),
            })
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn test_fetch_srtp_keys_from_mock_kms() {
    let server_url = spawn_mock_kms().await;
    let client = KmsClient::new(server_url).unwrap();

    let keys = client.fetch_srtp_keys(0xCAFE_BABE).await.unwrap();
    assert_eq!(keys.ssrc, 0xCAFE_BABE);
    assert_eq!(keys.master_key[0], 0x00);
    assert_eq!(keys.master_salt[0], 0x00);
}

#[test]
fn test_secret_is_masked_in_debug() {
    let key = SrtpKeyMaterial {
        ssrc: 0xCAFE_BABE,
        master_key: [0xAA; 16],
        master_salt: [0xBB; 14],
    };
    let debug_str = format!("{key:?}");
    assert!(!debug_str.contains("aa"));
    assert!(debug_str.contains("[REDACTED_16_BYTES]"));
}

