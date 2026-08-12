mod cli;
mod errors;
mod internals;

use clap::Parser;
use cli::Cli;
use std::fs::File;
use crate::internals::shared::stream_transaction_records;

fn main() -> anyhow::Result<(), errors::ApplicationError> {
    let args = Cli::try_parse()?;
    let file_path = args.file_path;
    let file = File::open(&file_path)?;
    let _ = stream_transaction_records(file);
    Ok(())
}
