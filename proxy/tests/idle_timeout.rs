use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

async fn start_echo_server() -> std::io::Result<std::net::SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        loop {
            let (mut s, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    let n = match s.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    if s.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            });
        }
    });

    Ok(addr)
}

// Minimal in-test proxy (matches your Day1 pattern)
async fn start_proxy(upstream: std::net::SocketAddr) -> std::io::Result<std::net::SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        loop {
            let (mut inbound, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut outbound = match TcpStream::connect(upstream).await {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
            });
        }
    });

    Ok(addr)
}

#[tokio::test]
async fn idle_connection_should_close_within_timeout() -> std::io::Result<()> {
    let upstream = start_echo_server().await?;
    let proxy = start_proxy(upstream).await?;

    let mut c = TcpStream::connect(proxy).await?;

    // Spec: idle connections should be closed within 200ms.
    // This will FAIL today (we have no idle timeout), and that's the point.
    let res = timeout(Duration::from_millis(200), async {
        let mut b = [0u8; 1];
        c.read_exact(&mut b).await
    }).await;

    assert!(
        res.is_ok(),
        "expected proxy to close idle connection within 200ms (spec test). Implement idle timeout."
    );

    Ok(())
}

