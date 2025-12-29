# bd-edge-rust

A learning-focused TCP proxy built with Rust and Tokio. This project is designed for hands-on exploration of async networking, connection handling, and test-driven development.

## Quick Start

```bash
# Run all tests (one will fail — that's intentional!)
cargo test -p proxy

# Run the proxy (listens on 9000, forwards to 9001)
cargo run -p proxy

# Run with custom addresses
cargo run -p proxy -- 127.0.0.1:8080 127.0.0.1:3000

# Run with debug logging
RUST_LOG=debug cargo run -p proxy
```

## Project Structure

```
bd-edge-rust/
├── proxy/           # The main TCP proxy
│   ├── src/main.rs  # ~60 lines of core proxy logic
│   └── tests/
│       ├── smoke.rs         # Basic functionality tests (PASS)
│       └── idle_timeout.rs  # Spec test for future feature (FAIL)
├── bench/           # Load testing tool
└── repro/           # Minimal reproduction cases
```

## Architecture

```
┌────────┐      ┌─────────┐      ┌──────────┐
│ Client │─────▶│  Proxy  │─────▶│ Upstream │
│        │◀─────│ :9000   │◀─────│  :9001   │
└────────┘      └─────────┘      └──────────┘
                     │
            copy_bidirectional()
            (handles both directions)
```

The proxy uses Tokio's `copy_bidirectional` which:
- Copies data in both directions simultaneously
- Handles backpressure automatically
- Propagates half-close correctly (FIN on one side triggers shutdown)

## Test Suite

| Test | Status | Purpose |
|------|--------|---------|
| `proxy_forwards_bytes_roundtrip` | ✅ PASS | Verifies basic proxy forwarding works |
| `half_close_client_can_still_read` | ✅ PASS | Verifies TCP half-close semantics |
| `idle_connection_should_close_within_timeout` | ❌ FAIL | **Spec test** — encodes a feature we haven't built yet |

### The Deliberately Failing Test

The `idle_timeout` test is a **spec test** — it defines behavior we *want* but haven't implemented:

```bash
cargo test -p proxy idle_connection -- --nocapture
```

**Expected failure:**
```
expected proxy to close idle connection within 200ms (spec test). Implement idle timeout.
```

This is your next implementation target. See [Implementation Guide](#implementing-idle-timeout) below.

## Key Concepts for Junior Devs

### 1. The Accept Loop Pattern

```rust
loop {
    let (socket, addr) = listener.accept().await?;  // Wait for connection
    tokio::spawn(async move {                        // Handle in background
        // ... handle connection ...
    });
}
```

This pattern allows handling many connections concurrently. Each `spawn` creates an independent task.

### 2. Why `copy_bidirectional`?

Instead of manually reading/writing in a loop, `copy_bidirectional`:
- Handles both directions (client→upstream and upstream→client) at once
- Applies backpressure (slows down fast senders if receiver is slow)
- Correctly propagates EOF/shutdown signals

```rust
// This one line does A LOT of work
tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
```

### 3. Half-Close (TCP FIN)

TCP connections have *two* directions that close independently:
1. Client can close its write side (sends FIN) but keep reading
2. Server can still send remaining data before closing its side

The `half_close` test verifies this works through the proxy.

### 4. Test Infrastructure Pattern

Tests create their own mini-servers on random ports (`127.0.0.1:0`):

```rust
let listener = TcpListener::bind("127.0.0.1:0").await?;  // OS picks port
let addr = listener.local_addr()?;                        // Get actual port
```

This avoids port conflicts between tests running in parallel.

## Implementing Idle Timeout

To make the failing test pass, you need to close connections that are idle (no data flowing) for too long.

**Approach:**
1. Replace `copy_bidirectional` with a manual copy loop
2. Use `tokio::select!` to race I/O against a timer
3. Reset the timer whenever data flows
4. Close connection if timer expires

**Skeleton:**

```rust
use tokio::time::{sleep, Duration, Instant};

let timeout_duration = Duration::from_millis(200);
let mut deadline = Instant::now() + timeout_duration;

loop {
    tokio::select! {
        // Race: I/O vs timeout
        result = /* read from one side */ => {
            // Handle data, reset deadline
            deadline = Instant::now() + timeout_duration;
        }
        _ = sleep_until(deadline) => {
            // Timeout expired, close connection
            break;
        }
    }
}
```

**Resources:**
- [tokio::select! macro](https://docs.rs/tokio/latest/tokio/macro.select.html)
- [copy_bidirectional source](https://docs.rs/tokio/latest/src/tokio/io/util/copy_bidirectional.rs.html) — see how it works internally

## Running the Benchmark

```bash
# Start an echo server on port 8080 first (or point proxy there)
cargo run -p bench
```

Outputs throughput stats like:
```
Completed in 1.23s: 100 success, 0 failed
Throughput: 81.30 connections/sec
```

## Development Workflow

```bash
# Watch tests (install cargo-watch first: cargo install cargo-watch)
cargo watch -x 'test -p proxy'

# Check without running
cargo check -p proxy

# Format code
cargo fmt

# Lint
cargo clippy
```

## Debugging Tips

1. **Enable logging:**
   ```bash
   RUST_LOG=debug cargo test -p proxy -- --nocapture
   ```

2. **Test a specific test:**
   ```bash
   cargo test -p proxy half_close -- --nocapture
   ```

3. **Use `nc` (netcat) to manually test:**
   ```bash
   # Terminal 1: Start a simple server
   nc -l 9001
   
   # Terminal 2: Start the proxy
   cargo run -p proxy
   
   # Terminal 3: Connect through proxy
   echo "hello" | nc localhost 9000
   ```

## Next Steps

1. ✅ Understand the current codebase (read `main.rs` — it's only 60 lines)
2. ✅ Run tests, see one fail
3. 🔲 Implement idle timeout to make the test pass
4. 🔲 Add more tests: connection limit, upstream timeout, graceful shutdown
5. 🔲 Add metrics (connection count, bytes transferred)

## Resources

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) — Start here for async Rust
- [copy_bidirectional docs](https://docs.rs/tokio/latest/tokio/io/fn.copy_bidirectional.html)
- [TCP half-close explained](https://stackoverflow.com/questions/37468816/what-is-tcp-half-close)
- [Rust async book](https://rust-lang.github.io/async-book/)

