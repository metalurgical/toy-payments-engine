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
use indexmap::IndexSet;
use std::fs::File;

fn main() -> anyhow::Result<(), errors::ApplicationError> {
    let args = Cli::try_parse()?;
    let file_path = args.file_path;
    let file = File::open(&file_path)?;
    let ledger = Ledger::new()?;
    let mut accounts_collection: IndexSet<ClientAccount> = IndexSet::new();

    let record_stream = stream_transaction_records(file);

    for record in record_stream {
        ledger.append_transaction(&record)?;
    }

    for client in ledger.retrieve_client_accounts() {
        accounts_collection.insert(ClientAccount::new(client));
    }

    for mut client in accounts_collection {
        for transaction in ledger
            .retrieve_all_transactions(client.client_id())
            .flatten()
        {
            let tx = transaction.clone();
            // eprintln!("{:?}", tx);
            match tx.transaction_type {
                TransactionType::Deposit => {
                    if let Err(e) = client.deposit(Transaction::new(tx)?) {
                        eprintln!("Skipping problematic transaction: {}", e);
                    }
                }
                TransactionType::Withdrawal => {
                    if let Err(e) = client.withdraw(Transaction::new(tx)?) {
                        eprintln!("Skipping problematic transaction: {}", e);
                    }
                }
                TransactionType::Dispute => {
                    if let Err(e) = client.dispute(Operation::new(tx)?) {
                        eprintln!("Skipping problematic transaction: {}", e);
                    }
                }
                TransactionType::Resolve => {
                    if let Err(e) = client.resolve(Operation::new(tx)?) {
                        eprintln!("Skipping problematic transaction: {}", e);
                    }
                }
                TransactionType::Chargeback => {
                    if let Err(e) = client.chargeback(Operation::new(tx)?) {
                        eprintln!("Skipping problematic transaction: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
