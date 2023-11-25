use std::env;

pub fn initialize() {
    if let Ok(filter) = env::var("RUST_LOG").or_else(|_| env::var("LOG")) {
        tracing_subscriber::FmtSubscriber::builder()
            .pretty()
            .without_time()
            .with_env_filter(filter)
            .init();
    }
}
