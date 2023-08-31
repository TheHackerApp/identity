use eyre::{eyre, WrapErr};
use std::{fs::OpenOptions, io::Write, path::PathBuf};
use tracing::info;

pub fn run(args: Args) -> eyre::Result<()> {
    if args.output.exists() && !args.force {
        return Err(eyre!("file already exists, use --force to overwrite"));
    }

    let mut output = OpenOptions::new()
        .write(true)
        .create(true)
        .create_new(!args.force)
        .open(&args.output)
        .wrap_err("failed to open output")?;

    let sdl = graphql::schema().sdl();
    output.write_all(sdl.as_bytes())?;

    info!(path = %args.output.display(), "successfully exported schema");

    Ok(())
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Where to save the schema
    #[arg(default_value = "./schema.graphql")]
    output: PathBuf,
    /// Whether to overwrite the output file if it already exists
    #[arg(short, long, default_value_t)]
    force: bool,
}
