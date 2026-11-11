use probe_core::kms_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kms_url = "http://localhost:8080";
    let ssrcs: Vec<u32> = (0x1000..0x1005).collect();

    Ok(())
}
