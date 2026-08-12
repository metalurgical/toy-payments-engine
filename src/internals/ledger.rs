use crate::errors::ApplicationError;
use crate::internals::transaction_record::TransactionRecord;
use sled::{Config, Db};

pub struct Ledger {
    db: Db,
}

impl Ledger {
    pub fn new() -> Result<Self, ApplicationError> {
        let db = Config::new()
            .temporary(true)
            .cache_capacity(64 * 1024 * 1024)
            .open()
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        Ok(Self { db })
    }

    pub fn append_transaction(&self, record: &TransactionRecord) -> Result<(), ApplicationError> {
        let generated_sequence_id = self
            .db
            .generate_id()
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        let prefix_len = 1;
        let client_bytes = record.client_id.to_be_bytes();
        let generate_sequence_byte = generated_sequence_id.to_be_bytes();
        let mut key =
            Vec::with_capacity(prefix_len + client_bytes.len() + generate_sequence_byte.len());
        key.push(b't');
        key.extend_from_slice(&client_bytes);
        key.extend_from_slice(&generate_sequence_byte);

        let value = postcard::to_allocvec(record)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        self.db
            .insert(key, value)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        Ok(())
    }

    pub fn retrieve_all_transactions(
        &self,
        client_id: u16,
    ) -> impl Iterator<Item = Result<TransactionRecord, ApplicationError>> {
        let mut prefix = Vec::with_capacity(3);
        prefix.push(b't');
        prefix.extend_from_slice(&client_id.to_be_bytes());

        self.db.scan_prefix(prefix).filter_map(|row| match row {
            Ok((_key, value)) => {
                let result = postcard::from_bytes(&value)
                    .map_err(|e| ApplicationError::LedgerError(e.to_string()));
                Some(result)
            }
            Err(_) => None,
        })
    }

    pub fn retrieve_client_accounts(&self) -> impl Iterator<Item = u16> {
        let mut current_search_key = vec![b't'];
        let db = self.db.clone();
        std::iter::from_fn(move || {
            if let Some(Ok((key, _value))) = db.range(current_search_key.clone()..).next()
                && key.starts_with(b"t")
                && key.len() >= 3
            {
                let mut client_bytes = [0u8; 2];
                client_bytes.copy_from_slice(&key[1..3]);
                let client_id = u16::from_be_bytes(client_bytes);
                if let Some(next_client_id) = client_id.checked_add(1) {
                    let mut next_key = vec![b't'];
                    next_key.extend_from_slice(&next_client_id.to_be_bytes());
                    current_search_key = next_key;
                } else {
                    current_search_key = vec![b'z']; // Overflow protection
                }
                return Some(client_id);
            }
            None
        })
    }
}

#[cfg(test)]
mod ledger_tests {
    use super::*;
    use crate::internals::transaction_type::TransactionType;

    fn default_ledger() -> Ledger {
        let ledger = Ledger::new().unwrap();

        let tx1 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: Some(1),
            amount: Some(20000),
        };

        let tx2 = TransactionRecord {
            transaction_type: TransactionType::Withdrawal,
            client_id: 1,
            transaction_id: Some(2),
            amount: Some(10000),
        };

        let tx3 = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: 2,
            transaction_id: Some(3),
            amount: Some(15000),
        };

        ledger.append_transaction(&tx1).unwrap();
        ledger.append_transaction(&tx2).unwrap();
        ledger.append_transaction(&tx3).unwrap();

        ledger
    }

    #[test]
    fn test_ledger_initialization() {
        let _ledger = Ledger::new().unwrap();
    }

    #[test]
    fn test_append_and_then_retrieve_transactions_by_client_id() {
        let ledger = default_ledger();
        let history: Vec<TransactionRecord> =
            ledger.retrieve_all_transactions(1).flatten().collect();

        assert_eq!(history.len(), 2);
        let history_0 = history[0].clone();
        let history_1 = history[1].clone();
        assert_eq!(history_0.transaction_id, Some(1));
        assert_eq!(history_1.transaction_id, Some(2));
        assert_eq!(history_0.transaction_type, TransactionType::Deposit);
        assert_eq!(history_1.transaction_type, TransactionType::Withdrawal);
        assert_eq!(history_0.amount, Some(20000));
        assert_eq!(history_1.amount, Some(10000));
    }

    #[test]
    fn test_retrieve_account_ids() {
        let ledger = default_ledger();
        let clients: Vec<u16> = ledger.retrieve_client_accounts().collect();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients, vec![1, 2]);
    }

    #[test]
    fn test_account_isolation_when_retrieving_history() {
        let ledger = Ledger::new().unwrap();

        let client_a = 1;
        let client_b = 2;

        let tx_a = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: client_a,
            transaction_id: Some(1),
            amount: Some(10000),
        };

        let tx_b = TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id: client_b,
            transaction_id: Some(2),
            amount: Some(20000),
        };

        ledger.append_transaction(&tx_a).unwrap();
        ledger.append_transaction(&tx_b).unwrap();

        let history_a: Vec<TransactionRecord> = ledger
            .retrieve_all_transactions(client_a)
            .flatten()
            .collect();
        assert_eq!(history_a.len(), 1);
        assert_eq!(history_a[0].transaction_id, Some(1));

        let history_b: Vec<TransactionRecord> = ledger
            .retrieve_all_transactions(client_b)
            .flatten()
            .collect();
        assert_eq!(history_b.len(), 1);
        assert_eq!(history_b[0].transaction_id, Some(2));
    }
}
