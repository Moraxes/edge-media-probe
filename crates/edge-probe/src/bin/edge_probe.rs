use std::net::SocketAddr;
//
use clap::{Parser, ValueEnum};
use edge_probe::{
  control_client::ControlClient, kms_client::KmsClient, orchestrator,
};
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum TraceBackend {
  Perfetto,
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

  #[arg(long, env = "EDGE_PROBE_TRACE", value_enum)]
  trace: Option<TraceBackend>,

  #[arg(long, env = "EDGE_PROBE_CONSOLE", default_value_t = false)]
  console: bool,

  #[arg(
    long,
    env = "EDGE_PROBE_TRACE_FILE",
    default_value = "edge-probe-trace.json"
  )]
  trace_file: String,
}

fn init_tracing(
  trace: Option<TraceBackend>,
  console: bool,
  trace_file: &str,
) -> Option<FlushGuard> {
  let console_layer = console.then(console_subscriber::spawn);
  match trace {
    Some(TraceBackend::Perfetto) => {
      let (chrome_layer, guard) = ChromeLayerBuilder::new()
        .file(trace_file)
        .build();
      tracing_subscriber::registry()
        .with(console_layer)
        .with(chrome_layer)
        .init();
      Some(guard)
    }
    Some(TraceBackend::Tracy) => {
      tracing_subscriber::registry()
        .with(console_layer)
        .with(tracing_tracy::TracyLayer::default())
        .init();
      None
    }
    None => {
      if let Some(layer) = console_layer {
        tracing_subscriber::registry()
          .with(layer)
          .init();
      }
      None
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let _trace_guard = init_tracing(args.trace, args.console, &args.trace_file);
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
