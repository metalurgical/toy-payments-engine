use crate::errors::ApplicationError;
use crate::internals::TransactionRecord;
use crate::internals::TransactionType;
use sled::{Config, Db, Tree};

#[derive(Clone)] // Cheap, sled handles the reference counting
pub struct Ledger {
    db: Db,
    deposited: Tree,
    withdrawn: Tree,
    disputed: Tree,
    resolved: Tree,
    charged_back: Tree,
}

impl Ledger {
    /// Opens a fresh temporary sled [`Db`], not shared between instances
    pub fn new() -> Result<Self, ApplicationError> {
        let db = Config::new()
            .temporary(true)
            .cache_capacity(64 * 1024 * 1024)
            .open()
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        // Create relevant tree for operations
        let open_tree = |name: &str| -> Result<Tree, ApplicationError> {
            db.open_tree(name)
                .map_err(|e| ApplicationError::LedgerError(e.to_string()))
        };

        Ok(Self {
            deposited: open_tree("deposits")?,
            withdrawn: open_tree("withdrawn")?,
            disputed: open_tree("disputed")?,
            resolved: open_tree("resolved")?,
            charged_back: open_tree("charged_back")?,
            db,
        })
    }

    fn index_key(client_id: u16, transaction_id: u32) -> [u8; 6] {
        let mut key = [0u8; 6];
        key[0..2].copy_from_slice(&client_id.to_be_bytes());
        key[2..6].copy_from_slice(&transaction_id.to_be_bytes());
        key
    }

    /// Returns the relevant operation [`Tree`]
    fn get_tree(&self, tx_type: TransactionType) -> &Tree {
        match tx_type {
            TransactionType::Withdrawal => &self.withdrawn,
            TransactionType::Dispute => &self.disputed,
            TransactionType::Resolve => &self.resolved,
            TransactionType::Chargeback => &self.charged_back,
            TransactionType::Deposit => &self.deposited,
        }
    }

    /// Checks if the relevant operation was already processed in the relevant tree
    /// TODO: Probably better to refactor arguments to Operation
    pub fn is_state(
        &self,
        tx_type: TransactionType,
        client_id: u16,
        transaction_id: u32,
    ) -> Result<bool, ApplicationError> {
        let key = Self::index_key(client_id, transaction_id);
        self.get_tree(tx_type)
            .contains_key(key)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))
    }

    /// Marks operation as processed in the relevant tree
    /// TODO: Probably better to refactor arguments to Operation
    pub fn mark_state(
        &self,
        tx_type: TransactionType,
        client_id: u16,
        transaction_id: u32,
    ) -> Result<(), ApplicationError> {
        let key = Self::index_key(client_id, transaction_id);
        let _ = self
            .get_tree(tx_type)
            .insert(key, &[] as &[u8])
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        Ok(())
    }

    /// Appends a [`TransactionRecord`] to the database
    pub fn append_transaction(&self, record: &TransactionRecord) -> Result<(), ApplicationError> {
        let prefix_len = 1;
        let client_bytes = record.client_id.to_be_bytes();
        let tx_id_bytes = record
            .transaction_id
            .ok_or_else(|| {
                ApplicationError::TransactionError("Missing required transaction_id".to_string())
            })?
            .to_be_bytes();
        let mut key = Vec::with_capacity(prefix_len + client_bytes.len() + tx_id_bytes.len());
        key.push(b't');
        key.extend_from_slice(&client_bytes);
        key.extend_from_slice(&tx_id_bytes);
        let value = postcard::to_allocvec(record)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        self.db
            .insert(key, value)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        Ok(())
    }

    /// Returns a [`TransactionRecord`] from the database
    pub fn get_transaction(
        &self,
        client_id: u16,
        transaction_id: u32,
    ) -> Result<Option<TransactionRecord>, ApplicationError> {
        let prefix_len = 1;
        let client_bytes = client_id.to_be_bytes();
        let tx_id_bytes = transaction_id.to_be_bytes();
        let mut key = Vec::with_capacity(prefix_len + client_bytes.len() + tx_id_bytes.len());
        key.push(b't');
        key.extend_from_slice(&client_bytes);
        key.extend_from_slice(&tx_id_bytes);
        let value_bytes = match self.db.get(&key) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(None),
            Err(e) => return Err(ApplicationError::LedgerError(e.to_string())),
        };
        let record: TransactionRecord = postcard::from_bytes(&value_bytes)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        Ok(Some(record))
    }
}

#[cfg(test)]
mod tests {
    use crate::internals::ledger::Ledger;
    use crate::internals::{TransactionRecord, TransactionType};

    fn record(client_id: u16, transaction_id: u32, amount: u128) -> TransactionRecord {
        TransactionRecord {
            transaction_type: TransactionType::Deposit,
            client_id,
            transaction_id: Some(transaction_id),
            amount: Some(amount),
        }
    }

    #[test]
    fn append_and_get_round_trips() {
        let ledger = Ledger::new().unwrap();
        let rec = record(1, 42, 12_345);
        ledger.append_transaction(&rec).unwrap();
        let fetched = ledger.get_transaction(1, 42).unwrap().unwrap();
        assert_eq!(fetched.client_id, 1);
        assert_eq!(fetched.transaction_id, Some(42));
        assert_eq!(fetched.amount, Some(12_345));
    }

    #[test]
    fn get_transaction_returns_none_when_missing() {
        let ledger = Ledger::new().unwrap();
        assert!(ledger.get_transaction(1, 999).unwrap().is_none());
    }

    #[test]
    fn append_transaction_requires_a_transaction_id() {
        let ledger = Ledger::new().unwrap();
        let mut rec = record(1, 1, 100);
        rec.transaction_id = None;
        assert!(ledger.append_transaction(&rec).is_err());
    }
    #[test]
    fn state_tracking_is_independent_per_client() {
        let ledger = Ledger::new().unwrap();
        ledger.mark_state(TransactionType::Deposit, 1, 1).unwrap();
        assert!(!ledger.is_state(TransactionType::Deposit, 2, 1).unwrap());
    }

    #[test]
    fn mark_state_works() {
        let ledger = Ledger::new().unwrap();
        assert!(!ledger.is_state(TransactionType::Dispute, 1, 1).unwrap());
        ledger.mark_state(TransactionType::Dispute, 1, 1).unwrap();
        assert!(ledger.is_state(TransactionType::Dispute, 1, 1).unwrap(),);
    }
}
