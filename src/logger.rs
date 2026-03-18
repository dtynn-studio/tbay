use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn logger_init() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
}
