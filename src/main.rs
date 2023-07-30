use anyhow::Result;
use zcash_primitives::consensus::Network;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args();
    let url = args.nth(1).expect("Need the data file URL as an argument");
    let fvk = args.nth(0).expect("Need the FVK as an argument");
    warp2::warp::scan::full_scan(&Network::MainNetwork, &url, &fvk, 0).await?;

    Ok(())
}

