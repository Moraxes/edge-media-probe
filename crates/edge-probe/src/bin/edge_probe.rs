use std::net::SocketAddr;
//
use clap::Parser;
use edge_probe::{control_client::ControlClient, orchestrator};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "KMS_URL", default_value = "http://127.0.0.1:8081")]
    kms_url: String,

    #[arg(long, env = "CONTROL_PLANE_URL", default_value = "http://127.0.0.1:8080")]
    control_plane: String,

    #[arg(long, default_value = "127.0.0.1:5004")]
    listen: SocketAddr,

    #[arg(long)]
    forward: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let control = ControlClient::new(&args.control_plane);
    orchestrator::run_probe(&args.kms_url, &control, args.listen, args.forward).await?;
    Ok(())
}
