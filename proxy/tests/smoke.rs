use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Starts a simple echo server that sends back whatever it receives.
/// Returns the address the server is listening on.
/// 
/// The server runs in a background task and will accept multiple concurrent connections.
async fn start_echo_server() -> std::io::Result<std::net::SocketAddr> {
    // Bind to a random available port on localhost (port 0 means "pick any free port")
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Spawn a background task to handle incoming connections
    tokio::spawn(async move {
        loop {
            // Accept a new connection
            let (mut s, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break, // Stop accepting if there's an error
            };
            // Spawn a separate task for each connection so we can handle multiple clients
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    // Read data from the client
                    let n = match s.read(&mut buf).await {
                        Ok(0) => break, // Connection closed
                        Ok(n) => n,     // Got n bytes
                        Err(_) => break, // Read error
                    };
                    // Echo the data back to the client
                    if s.write_all(&buf[..n]).await.is_err() {
                        break; // Write error, close connection
                    }
                }
            });
        }
    });

    Ok(addr)
}

/// Starts a TCP proxy that forwards all traffic to the upstream server.
/// Returns the address the proxy is listening on.
/// 
/// This is the core proxy functionality: it accepts connections and forwards
/// all data bidirectionally between the client and the upstream server.
async fn start_proxy(upstream: std::net::SocketAddr) -> std::io::Result<std::net::SocketAddr> {
    // Bind to a random available port on localhost
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Spawn a background task to handle incoming connections
    tokio::spawn(async move {
        loop {
            // Accept a connection from a client
            let (mut inbound, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break,
            };

            // Spawn a task to handle this specific client connection
            tokio::spawn(async move {
                // Connect to the upstream server
                let mut outbound = match TcpStream::connect(upstream).await {
                    Ok(s) => s,
                    Err(_) => return, // If we can't connect upstream, give up
                };
                // Copy data bidirectionally: client <-> proxy <-> upstream
                // This handles both directions simultaneously
                let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
            });
        }
    });

    Ok(addr)
}

/// Integration test: verify that the proxy correctly forwards data in both directions.
/// 
/// Test flow:
/// 1. Start an echo server (upstream)
/// 2. Start a proxy pointing to the echo server
/// 3. Connect a client to the proxy
/// 4. Send data through the proxy and verify it comes back echoed
#[tokio::test]
async fn proxy_forwards_bytes_roundtrip() -> std::io::Result<()> {
    // Set up the test infrastructure
    let upstream = start_echo_server().await?;
    let proxy = start_proxy(upstream).await?;

    // Connect to the proxy (not directly to the echo server)
    let mut c = TcpStream::connect(proxy).await?;
    
    // Send a test message
    c.write_all(b"ping").await?;

    // Read the response
    let mut out = [0u8; 4];
    c.read_exact(&mut out).await?;
    
    // Verify we got back what we sent (proving the proxy works)
    assert_eq!(&out, b"ping");

    Ok(())
}

/// Test TCP half-close semantics: client closes write side but can still read response.
/// 
/// This verifies that copy_bidirectional correctly handles the case where one direction
/// is closed (client sends FIN) while the other direction still has data to deliver.
#[tokio::test]
async fn half_close_client_can_still_read() -> std::io::Result<()> {
    let upstream = start_echo_server().await?;
    let proxy = start_proxy(upstream).await?;

    let mut c = TcpStream::connect(proxy).await?;
    c.write_all(b"ping").await?;
    
    // Half-close: client closes write side but keeps read open
    c.shutdown().await?;

    // Should still receive the echoed response
    let mut out = [0u8; 4];
    c.read_exact(&mut out).await?;
    assert_eq!(&out, b"ping");

    Ok(())
}
