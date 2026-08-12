mod cli;
mod errors;
mod internals;

use crate::internals::client_account::ClientAccount;
use crate::internals::ledger::Ledger;
use crate::internals::shared::stream_transaction_records;
use clap::Parser;
use cli::Cli;
use std::collections::BTreeMap;
use std::fs::File;

fn main() -> anyhow::Result<(), errors::ApplicationError> {
    let args = Cli::try_parse()?;
    let file_path = args.file_path;
    let file = File::open(&file_path)?;
    let ledger = Ledger::new()?;
    let mut accounts_collection: BTreeMap<u16, ClientAccount> = BTreeMap::new();

    let record_stream = stream_transaction_records(file);

    for record in record_stream {
        accounts_collection
            .entry(record.client_id)
            .or_insert(ClientAccount::new(record.client_id));
        ledger.append_transaction(&record)?;
    }
    Ok(())
}
