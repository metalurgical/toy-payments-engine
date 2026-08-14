mod cli;
mod engine;
mod errors;
mod internals;
mod logging;

use crate::{
    engine::run_engine, errors::ApplicationError, internals::Ledger, logging::init_logging,
};
use clap::Parser;
use cli::Cli;
use std::fs::File;

#[tokio::main]
async fn main() -> anyhow::Result<(), ApplicationError> {
    init_logging()?;
    let args = Cli::try_parse()?;
    let file = File::open(&args.file_path)?;
    let ledger = Ledger::new()?;
    let accounts = run_engine(file, ledger).await?;
    let mut csv_writer = csv::Writer::from_writer(std::io::stdout());
    for client in &accounts {
        csv_writer.serialize(client)?;
    }
    csv_writer.flush()?;
    Ok(())
}
