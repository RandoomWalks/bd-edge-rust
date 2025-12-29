// Import Tokio's async I/O utilities for bidirectional copying between streams
use tokio::io::copy_bidirectional;
// Import Tokio's TCP networking primitives for listening and connecting
use tokio::net::{TcpListener, TcpStream};
// Import tracing macros for structured logging
use tracing::{info, error};
// Import environment filter for configuring log levels via RUST_LOG env var
use tracing_subscriber::EnvFilter;

// Main entry point - tokio::main macro sets up the async runtime
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tracing subscriber for logging
    // This reads the RUST_LOG environment variable to set log levels
    // Example: RUST_LOG=debug cargo run
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse command line arguments for listen and upstream addresses
    // Usage: proxy <LISTEN_ADDR:PORT> <UPSTREAM_ADDR:PORT>
    let mut args = std::env::args().skip(1);
    // Default to localhost:9000 if no listen address provided
    let listen = args.next().unwrap_or_else(|| "127.0.0.1:9000".into());
    // Default to localhost:9001 if no upstream address provided
    let upstream = args.next().unwrap_or_else(|| "127.0.0.1:9001".into());

    // Bind to the listen address and start accepting connections
    let listener = TcpListener::bind(&listen).await?;
    info!(%listen, %upstream, "Proxy listening, forwarding to upstream");

    // Main accept loop - runs forever accepting new connections
    loop {
        // Wait for a new client connection
        // Returns the client socket and their address
        let (mut client, addr) = listener.accept().await?;
        info!(%addr, "Accepted connection");
        // Clone the upstream address for this connection's task
        let upstream = upstream.clone();

        // Spawn a new async task to handle this connection independently
        // This allows the proxy to handle multiple connections concurrently
        tokio::spawn(async move {
            // Connect to upstream and use copy_bidirectional for backpressure
            // copy_bidirectional efficiently copies data in both directions simultaneously
            // and handles backpressure automatically (slows down if one side can't keep up)
            match TcpStream::connect(&upstream).await {
                Ok(mut upstream) => {
                    // Successfully connected - now proxy data bidirectionally
                    // This will run until either side closes the connection or an error occurs
                    if let Err(e) = copy_bidirectional(&mut client, &mut upstream).await {
                        error!(%addr, error = %e, "Connection error");
                    }
                }
                Err(e) => {
                    // Failed to connect to upstream server
                    error!(%addr, %upstream, error = %e, "Failed to connect to upstream");
                }
            }
            // Task ends here - both connections are automatically closed when dropped
        });
    }
}
