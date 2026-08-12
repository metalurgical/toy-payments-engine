mod cli;
mod errors;
mod internals;

use crate::internals::transaction_record::TransactionRecord;
use clap::Parser;
use cli::Cli;
use std::fs::File;

pub fn read_transaction_records(file: File) -> Vec<TransactionRecord> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(file);

    let mut records = Vec::new();
    for result in reader.deserialize::<TransactionRecord>() {
        match result {
            Ok(record) => records.push(record),
            Err(e) => {
                // Log and drop problematic rows
                eprintln!("Skipping problematic item: {}", e);
            }
        }
    }
    records
}

fn main() -> anyhow::Result<(), errors::ApplicationError> {
    let args = Cli::try_parse()?;
    let file_path = args.file_path;
    let file = File::open(&file_path)?;
    let _ = read_transaction_records(file);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internals::transaction_type::TransactionType;
    use std::path::PathBuf;
    #[test]
    fn test_parse_csv_to_transaction_record() {
        let file = File::open(PathBuf::from("test_input/basic_test.csv")).unwrap();
        let records = read_transaction_records(file);
        assert_eq!(records.len(), 9);

        let deposit = records[0].clone();
        assert_eq!(deposit.transaction_type, TransactionType::Deposit);
        assert_eq!(deposit.client_id, 1);
        assert_eq!(deposit.transaction_id, Some(1));
        assert_eq!(deposit.amount, Some(10_000));

        let withdrawal = records[3].clone();
        assert_eq!(withdrawal.transaction_type, TransactionType::Withdrawal);
        assert_eq!(withdrawal.client_id, 1);
        assert_eq!(withdrawal.transaction_id, Some(4));
        assert_eq!(withdrawal.amount, Some(15_000));

        let dispute = records[5].clone();
        assert_eq!(dispute.transaction_type, TransactionType::Dispute);
        assert_eq!(dispute.client_id, 2);
        assert_eq!(dispute.transaction_id, Some(2));
        assert_eq!(dispute.amount, None);

        let resolve = records[6].clone();
        assert_eq!(resolve.transaction_type, TransactionType::Resolve);
        assert_eq!(resolve.client_id, 2);
        assert_eq!(resolve.transaction_id, Some(2));
        assert_eq!(resolve.amount, None);

        let chargeback = records[8].clone();
        assert_eq!(chargeback.transaction_type, TransactionType::Chargeback);
        assert_eq!(chargeback.client_id, 1);
        assert_eq!(chargeback.transaction_id, Some(1));
        assert_eq!(chargeback.amount, None);
    }

    #[test]
    fn test_parse_csv_skips_bad_rows() {
        let file = File::open(PathBuf::from("test_input/basic_test.csv")).unwrap();
        let records = read_transaction_records(file);
        assert_eq!(records.len(), 9);
    }
}
