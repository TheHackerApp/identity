use eyre::WrapErr;

pub mod logging;
mod otel;

/// Load environment variables from a .env file, if it exists.
pub fn dotenv() -> eyre::Result<()> {
    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            return Err(error).wrap_err("failed to load .env");
        }
    }

    Ok(())
}
