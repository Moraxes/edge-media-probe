//! Klient HTTP dla płaszczyzny sterującej (`control-plane`) ze współdzieloną pulą połączeń.

use reqwest::Client;
//
use crate::control_plane::{SlaAlertPayload, StreamInfo};

#[derive(Debug, Clone)]
pub struct ControlClient {
    client: Client,
    base_url: String,
}

impl ControlClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Zadanie 6: zarejestruj strumień przez `POST {base_url}/v1/streams`.
    pub async fn register_stream(&self, stream: &StreamInfo) -> Result<(), reqwest::Error> {
        let url = format!("{}/v1/streams", self.base_url);
        self.client
            .post(url)
            .json(stream)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Zadanie 6: pobierz listę zarejestrowanych strumieni z `GET {base_url}/v1/streams`.
    pub async fn fetch_streams(&self) -> Result<Vec<StreamInfo>, reqwest::Error> {
        let url = format!("{}/v1/streams", self.base_url);
        self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    /// Zadanie 6: wyślij alert SLA przez `POST {base_url}/v1/alerts`.
    pub async fn send_alert(&self, alert: &SlaAlertPayload) -> Result<(), reqwest::Error> {
        let url = format!("{}/v1/alerts", self.base_url);
        self.client
            .post(url)
            .json(alert)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
