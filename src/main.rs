mod cli;
mod errors;
mod internals;

use clap::Parser;
use cli::Cli;
use std::fs::File;

fn main() -> anyhow::Result<(), errors::ApplicationError> {
    let args = Cli::try_parse()?;
    let file_path = args.file_path;
    let file = File::open(&file_path)?;
    let mut _reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file);
    Ok(())
}
