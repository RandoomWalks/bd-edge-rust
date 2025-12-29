//! Performance testing harness for the edge proxy.
//!
//! This benchmark tool helps measure the proxy's throughput and connection handling.
//! It spawns multiple concurrent connections to stress-test the system.
//!
//! Run with: cargo run -p bench
//! Profile with: cargo flamegraph -p bench

use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

/// The address of the proxy server we're testing
const TARGET_ADDR: &str = "127.0.0.1:8080";

/// How many concurrent connections to open during the test
const NUM_CONNECTIONS: usize = 100;

/// Size of the test payload we send (in bytes)
const PAYLOAD_SIZE: usize = 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging - set RUST_LOG=debug for verbose output
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting load test against {}", TARGET_ADDR);
    info!(
        "Connections: {}, Payload size: {} bytes",
        NUM_CONNECTIONS, PAYLOAD_SIZE
    );

    // Start timing the entire test
    let start = Instant::now();
    let mut handles = Vec::with_capacity(NUM_CONNECTIONS);

    // Spawn all connections concurrently as separate async tasks
    // Each task runs independently and can make progress in parallel
    for i in 0..NUM_CONNECTIONS {
        handles.push(tokio::spawn(async move {
            run_connection(i).await
        }));
    }

    // Wait for all tasks to complete and count successes/failures
    let mut success = 0;
    let mut failed = 0;

    for handle in handles {
        match handle.await {
            // Task completed successfully
            Ok(Ok(_)) => success += 1,
            // Task completed but connection had an error
            Ok(Err(e)) => {
                warn!("Connection error: {}", e);
                failed += 1;
            }
            // Task panicked (shouldn't happen in normal operation)
            Err(e) => {
                warn!("Task panic: {}", e);
                failed += 1;
            }
        }
    }

    // Report results
    let elapsed = start.elapsed();
    info!(
        "Completed in {:?}: {} success, {} failed",
        elapsed, success, failed
    );
    info!(
        "Throughput: {:.2} connections/sec",
        NUM_CONNECTIONS as f64 / elapsed.as_secs_f64()
    );

    Ok(())
}

/// Runs a single connection test: connect, send data, read response
///
/// # Arguments
/// * `id` - Connection identifier for logging purposes
///
/// # Returns
/// * `Ok(())` if the connection completed successfully
/// * `Err` if there was a network error
async fn run_connection(id: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Establish TCP connection to the proxy
    let mut stream = TcpStream::connect(TARGET_ADDR).await?;

    // Send test payload - just a bunch of 'x' characters
    let payload = vec![b'x'; PAYLOAD_SIZE];
    stream.write_all(&payload).await?;

    // Try to read response from the server
    // We use a timeout to avoid hanging if the server doesn't respond
    let mut buf = vec![0u8; PAYLOAD_SIZE];
    match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await {
        Ok(Ok(n)) => {
            // Successfully read n bytes
            tracing::debug!(id, bytes = n, "Received response");
        }
        Ok(Err(e)) => {
            // Read operation failed (connection closed, etc.)
            tracing::debug!(id, error = %e, "Read error");
        }
        Err(_) => {
            // Timeout - server didn't respond within 5 seconds
            tracing::debug!(id, "Read timeout");
        }
    }

    Ok(())
}
