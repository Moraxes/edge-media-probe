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
  pub fn new(_base_url: &str) -> Self {
    todo!()
  }

  /// Zadanie 6: zarejestruj strumień przez `POST {base_url}/v1/streams`.
  pub async fn register_stream(
    &self,
    _stream: &StreamInfo,
  ) -> Result<(), reqwest::Error> {
    todo!()
  }

  /// Zadanie 6: pobierz listę zarejestrowanych strumieni z `GET {base_url}/v1/streams`.
  pub async fn fetch_streams(&self) -> Result<Vec<StreamInfo>, reqwest::Error> {
    todo!()
  }

  /// Zadanie 6: wyślij alert SLA przez `POST {base_url}/v1/alerts`.
  pub async fn send_alert(
    &self,
    _alert: &SlaAlertPayload,
  ) -> Result<(), reqwest::Error> {
    todo!()
  }
}
