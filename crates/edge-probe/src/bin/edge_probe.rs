use std::net::SocketAddr;
//
use clap::Parser;
use edge_probe::{
  control_client::ControlClient, kms_client::KmsClient, orchestrator,
};

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
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
