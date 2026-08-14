use crate::errors::ApplicationError;
use crate::internals::{
    client_account::ClientAccount, ledger::Ledger, operation::Operation,
    shared::stream_transaction_records, transaction::Transaction,
    transaction_record::TransactionRecord, transaction_type::TransactionType,
};
use log::{error, warn};
use std::{collections::HashMap, fs::File};
use tokio::{sync::mpsc, task::JoinSet};

const PER_CLIENT_CHANNEL_CAPACITY: usize = 64;

fn apply_record(client: &mut ClientAccount, tx: TransactionRecord) {
    let result = match tx.transaction_type {
        TransactionType::Deposit => Transaction::new(tx).and_then(|t| client.deposit(t)),
        TransactionType::Withdrawal => Transaction::new(tx).and_then(|t| client.withdraw(t)),
        TransactionType::Dispute => Operation::new(tx).and_then(|op| client.dispute(op)),
        TransactionType::Resolve => Operation::new(tx).and_then(|op| client.resolve(op)),
        TransactionType::Chargeback => Operation::new(tx).and_then(|op| client.chargeback(op)),
    };
    if let Err(e) = result {
        warn!("Skipping problematic transaction: {}", e);
    }
}

pub async fn run_engine(
    file: File,
    ledger: Ledger,
) -> Result<Vec<ClientAccount>, ApplicationError> {
    let mut client_tasks: JoinSet<ClientAccount> = JoinSet::new();
    let mut senders: HashMap<u16, mpsc::Sender<TransactionRecord>> = HashMap::new();
    for record in stream_transaction_records(file) {
        match record.transaction_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if let Err(e) = ledger.append_transaction(&record) {
                    error!("Failed to append transaction to ledger, skipping: {}", e);
                    continue;
                }
            }
            _ => {}
        }
        let client_id = record.client_id;
        let sender = senders.entry(client_id).or_insert_with(|| {
            let (tx, mut rx) = mpsc::channel::<TransactionRecord>(PER_CLIENT_CHANNEL_CAPACITY);
            let ledger = ledger.clone();
            client_tasks.spawn(async move {
                let mut client = ClientAccount::new(client_id, ledger);
                while let Some(record) = rx.recv().await {
                    apply_record(&mut client, record);
                }
                client
            });
            tx
        });
        if sender.send(record).await.is_err() {
            error!("Channel died for client {}", client_id);
        }
    }
    drop(senders);
    let mut accounts = Vec::new();
    while let Some(result) = client_tasks.join_next().await {
        match result {
            Ok(client) => accounts.push(client),
            Err(join_err) => {
                error!("Skipping client join error during processing: {}", join_err);
            }
        }
    }
    accounts.sort_by_key(|c| c.client_id());
    Ok(accounts)
}
