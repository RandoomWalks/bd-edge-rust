//! Repro crate for minimal issue reproductions.
//!
//! Each issue gets its own binary in `src/bin/`:
//! - `src/bin/issue_001.rs` -> `cargo run -p repro --bin issue_001`
//!
//! Shared helpers can go in this lib.rs.

/// Common setup for repro binaries
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

