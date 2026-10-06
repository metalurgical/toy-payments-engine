use crate::errors::ApplicationError;
use crate::internals::{
    ClientAccount, Ledger, Operation, Transaction, TransactionRecord, TransactionType,
    shared::stream_transaction_records,
};
use log::{error, warn};
use std::{collections::HashMap, fs::File};
use tokio::{sync::mpsc, task::JoinSet};

/// Bounded capacity of each worker channel
const WORKER_CHANNEL_CAPACITY: usize = 1024;

/// Number of worker tasks
fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Applies [`TransactionRecord`] to [`ClientAccount`], converting the [`TransactionRecord`] to the respective
/// [`Transaction`] or [`Operation`] DTO and then calling the respective [`ClientAccount`] method determined
/// by [`TransactionType`]
fn apply_record(client: &mut ClientAccount, tx: TransactionRecord) {
    let result = match tx.transaction_type {
        TransactionType::Deposit => Transaction::new(tx).and_then(|t| client.deposit(t)),
        TransactionType::Withdrawal => Transaction::new(tx).and_then(|t| client.withdraw(t)),
        TransactionType::Dispute => Operation::new(tx).and_then(|op| client.dispute(op)),
        TransactionType::Resolve => Operation::new(tx).and_then(|op| client.resolve(op)),
        TransactionType::Chargeback => Operation::new(tx).and_then(|op| client.chargeback(op)),
    };
    if let Err(e) = result {
        // log error
        warn!("Skipping problematic transaction: {}", e);
    }
}

/// Processes the CSV file, adding valid entries to [`Ledger`] as well as creating and
/// applying operations to [`ClientAccount`]. Returns `Vec<ClientAccount>` for all successfully
/// processed accounts. Records are sharded across a fixed pool of workers by client id.
pub async fn run_engine(
    file: File,
    ledger: Ledger,
) -> Result<Vec<ClientAccount>, ApplicationError> {
    let workers = worker_count();
    let mut worker_tasks: JoinSet<Vec<ClientAccount>> = JoinSet::new();
    let mut senders: Vec<mpsc::Sender<TransactionRecord>> = Vec::with_capacity(workers);
    for _ in 0..workers {
        let (tx, mut rx) = mpsc::channel::<TransactionRecord>(WORKER_CHANNEL_CAPACITY);
        let ledger = ledger.clone();
        worker_tasks.spawn_blocking(move || {
            let mut clients: HashMap<u16, ClientAccount> = HashMap::new();
            while let Some(record) = rx.blocking_recv() {
                let client = clients
                    .entry(record.client_id)
                    .or_insert_with(|| ClientAccount::new(record.client_id, ledger.clone()));
                apply_record(client, record);
            }
            clients.into_values().collect()
        });
        senders.push(tx);
    }
    for record in stream_transaction_records(file) {
        let client_id = record.client_id;
        let sender = &senders[usize::from(client_id) % workers];
        if sender.send(record).await.is_err() {
            error!(
                "Worker channel closed, dropping record for client {}",
                client_id
            );
        }
    }
    drop(senders);
    let mut accounts = Vec::new();
    while let Some(result) = worker_tasks.join_next().await {
        match result {
            Ok(worker_accounts) => accounts.extend(worker_accounts),
            Err(join_err) => {
                error!("Skipping worker join error during processing: {}", join_err);
            }
        }
    }
    // Sort client accounts
    accounts.sort_by_key(|c| c.client_id());
    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env::temp_dir, fs, io::Write, process, sync::mpsc::channel, time::Duration};
    use tokio::runtime::Builder;

    #[test]
    fn test_many_clients_does_not_deadlock() {
        {
            let clients: u32 = 600;
            let rounds: u32 = 700;
            let path = temp_dir().join(format!("many_clients_{}.csv", process::id()));
            let mut f = File::create(&path).unwrap();
            writeln!(f, "type,client,tx,amount").unwrap();
            let mut tx = 1;
            for _ in 0..rounds {
                for c in 1..=clients {
                    writeln!(f, "deposit,{c},{tx},1.0").unwrap();
                    tx += 1;
                }
            }
            let file = File::open(&path).unwrap();
            let (done_tx, done_rx) = channel();
            let rt = Builder::new_multi_thread().build().unwrap();
            let accounts = rt.block_on(run_engine(file, Ledger::new().unwrap()));
            let _ = done_tx.send(accounts.map(|a| a.len()));
            let result = done_rx.recv_timeout(Duration::from_secs(60)).unwrap();
            fs::remove_file(&path).ok();
            assert_eq!(result.unwrap(), clients as usize);
        }
    }
}
