use std::env;

/// Initialize logging based on environment filter (LOG or RUST_LOG)
pub fn initialize() {
    let filter = env::var("RUST_LOG").or_else(|_| env::var("LOG"));
    if let Ok(filter) = filter {
        tracing_subscriber::fmt()
            .pretty()
            .without_time()
            .with_env_filter(filter)
            .init();
    }
}
