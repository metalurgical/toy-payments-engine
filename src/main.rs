mod cli;
mod errors;
mod internals;

use crate::internals::client_account::ClientAccount;
use crate::internals::ledger::Ledger;
use crate::internals::operation::Operation;
use crate::internals::shared::stream_transaction_records;
use crate::internals::transaction::Transaction;
use crate::internals::transaction_type::TransactionType;
use clap::Parser;
use cli::Cli;
use indexmap::IndexMap;
use std::fs::File;

fn main() -> anyhow::Result<(), errors::ApplicationError> {
    let args = Cli::try_parse()?;
    let file_path = args.file_path;
    let file = File::open(&file_path)?;
    let ledger = Ledger::new()?;
    let mut accounts_collection: IndexMap<u16, ClientAccount> = IndexMap::new();

    let record_stream = stream_transaction_records(file);

    for record in record_stream {
        ledger.append_transaction(&record)?;
    }

    for client_id in ledger.retrieve_client_accounts() {
        accounts_collection
            .entry(client_id)
            .or_insert_with(|| ClientAccount::new(client_id));
    }

    for (_, client) in accounts_collection.iter_mut() {
        for transaction in ledger
            .retrieve_all_transactions(client.client_id())
            .flatten()
        {
            let tx = transaction.clone();
            match tx.transaction_type {
                TransactionType::Deposit => match Transaction::new(tx) {
                    Ok(tx) => {
                        if let Err(e) = client.deposit(tx) {
                            eprintln!("Skipping problematic transaction: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Skipping problematic transaction: {}", e),
                },
                TransactionType::Withdrawal => match Transaction::new(tx) {
                    Ok(tx) => {
                        if let Err(e) = client.withdraw(tx) {
                            eprintln!("Skipping problematic transaction: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Skipping problematic transaction: {}", e),
                },
                TransactionType::Dispute => match Operation::new(tx) {
                    Ok(op) => {
                        if let Err(e) = client.dispute(op) {
                            eprintln!("Skipping problematic transaction: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Skipping problematic transaction: {}", e),
                },
                TransactionType::Resolve => match Operation::new(tx) {
                    Ok(op) => {
                        if let Err(e) = client.resolve(op) {
                            eprintln!("Skipping problematic transaction: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Skipping problematic transaction: {}", e),
                },
                TransactionType::Chargeback => match Operation::new(tx) {
                    Ok(op) => {
                        if let Err(e) = client.chargeback(op) {
                            eprintln!("Skipping problematic transaction: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Skipping problematic transaction: {}", e),
                },
            }
        }
    }

    let mut csv_writer = csv::Writer::from_writer(std::io::stdout());
    for client in accounts_collection.values() {
        csv_writer.serialize(client)?;
    }
    csv_writer.flush()?;

    Ok(())
}
