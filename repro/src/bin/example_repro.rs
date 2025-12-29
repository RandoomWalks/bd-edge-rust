//! Example repro template.
//! Copy this file and rename for each new issue reproduction.
//!
//! Run with: cargo run -p repro --bin example_repro

use repro::init_tracing;
use tracing::info;

#[tokio::main]
async fn main() {
    init_tracing();
    info!("Running example repro");

    // TODO: Add minimal reproduction code here
}

