use anyhow::Result;
use zcash_primitives::consensus::Network;
use crate::warp::scan::full_scan;

#[path = "cash.z.wallet.sdk.rpc.rs"]
pub mod lw_rpc;
pub mod sapling;
pub mod warp;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args();
    let fvk = args.nth(1).expect("Need the FVK as an argument");
    full_scan(&Network::MainNetwork, &fvk).await?;

    Ok(())
}
