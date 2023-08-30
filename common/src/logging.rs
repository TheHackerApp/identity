use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Setup logging and error reporting
pub fn init(default_level: Level) {
    let debug = cfg!(debug_assertions);

    Registry::default()
        .with(
            EnvFilter::builder()
                .with_default_directive(default_level.into())
                .from_env_lossy(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(debug)
                .with_line_number(debug)
                .with_target(true),
        )
        .with(ErrorLayer::default())
        .init()
}
