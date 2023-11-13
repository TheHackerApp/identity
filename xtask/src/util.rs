use eyre::WrapErr;
use redis::aio::ConnectionManager;
use sqlx::{
    postgres::{PgConnectOptions, PgPool},
    ConnectOptions,
};
use std::str::FromStr;
use tracing::{info, log::LevelFilter};

/// Connect to a cache instance
pub async fn connect_to_cache(url: &str) -> eyre::Result<ConnectionManager> {
    let client = redis::Client::open(url).wrap_err("invalid cache URL format")?;
    let cache = client
        .get_tokio_connection_manager()
        .await
        .wrap_err("failed to connect to the cache")?;

    info!("connected to the cache");

    Ok(cache)
}

/// Connect to the database
pub async fn connect_to_database(url: &str) -> eyre::Result<PgPool> {
    let options = PgConnectOptions::from_str(url)
        .wrap_err("invalid database URL format")?
        .log_statements(LevelFilter::Debug);
    let db = PgPool::connect_with(options)
        .await
        .wrap_err("failed to connect to the database")?;

    info!("connected to the database");

    Ok(db)
}
