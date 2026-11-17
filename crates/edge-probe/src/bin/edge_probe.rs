use std::net::SocketAddr;
//
use clap::{Parser, ValueEnum};
use edge_probe::{
  control_client::ControlClient, kms_client::KmsClient, orchestrator,
};
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum DiagMode {
  #[default]
  None,
  Console,
  Chrome,
  Tracy,
}

#[derive(Debug, Parser)]
struct Args {
  #[arg(long, env = "KMS_URL", default_value = "http://127.0.0.1:8081")]
  kms_url: String,

  #[arg(
    long,
    env = "CONTROL_PLANE_URL",
    default_value = "http://127.0.0.1:8080"
  )]
  control_plane: String,

  #[arg(long, default_value = "127.0.0.1:5004")]
  listen: SocketAddr,

  #[arg(long, default_value = "127.0.0.1:5006")]
  forward: SocketAddr,

  #[arg(long, default_value_t = 1)]
  workers: usize,

  #[arg(long, env = "EDGE_PROBE_DIAG", value_enum, default_value_t = DiagMode::None)]
  diag: DiagMode,

  #[arg(
    long,
    env = "EDGE_PROBE_TRACE_FILE",
    default_value = "edge-probe-trace.json"
  )]
  trace_file: String,
}

fn init_diag(mode: DiagMode, trace_file: &str) -> Option<FlushGuard> {
  match mode {
    DiagMode::None => None,
    DiagMode::Console => {
      console_subscriber::init();
      None
    }
    DiagMode::Chrome => {
      let (chrome_layer, guard) = ChromeLayerBuilder::new()
        .file(trace_file)
        .build();
      tracing_subscriber::registry()
        .with(chrome_layer)
        .init();
      Some(guard)
    }
    DiagMode::Tracy => {
      tracing_subscriber::registry()
        .with(tracing_tracy::TracyLayer::default())
        .init();
      None
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let _diag_guard = init_diag(args.diag, &args.trace_file);
  let kms = KmsClient::new(&args.kms_url);
  let control = ControlClient::new(&args.control_plane);
  if args.workers > 1 {
    orchestrator::run_probe_workers(
      &kms,
      &control,
      args.listen,
      args.forward,
      args.workers,
    )
    .await?;
  } else {
    orchestrator::run_probe(&kms, &control, args.listen, args.forward).await?;
  }
  Ok(())
}
