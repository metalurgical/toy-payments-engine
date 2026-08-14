use crate::errors::ApplicationError;
use crate::internals::transaction_record::TransactionRecord;
use crate::internals::transaction_type::TransactionType;
use sled::{Config, Db, Tree};

#[derive(Clone)]
pub struct Ledger {
    db: Db,
    deposited: Tree,
    withdrawn: Tree,
    disputed: Tree,
    resolved: Tree,
    charged_back: Tree,
}

impl Ledger {
    pub fn new() -> Result<Self, ApplicationError> {
        let db = Config::new()
            .temporary(true)
            .cache_capacity(64 * 1024 * 1024)
            .open()
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;

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

    fn get_tree(&self, tx_type: TransactionType) -> Result<&Tree, ApplicationError> {
        match tx_type {
            TransactionType::Withdrawal => Ok(&self.withdrawn),
            TransactionType::Dispute => Ok(&self.disputed),
            TransactionType::Resolve => Ok(&self.resolved),
            TransactionType::Chargeback => Ok(&self.charged_back),
            TransactionType::Deposit => Ok(&self.deposited),
        }
    }

    pub fn is_state(
        &self,
        tx_type: TransactionType,
        client_id: u16,
        transaction_id: u32,
    ) -> Result<bool, ApplicationError> {
        let key = Self::index_key(client_id, transaction_id);
        self.get_tree(tx_type)?
            .contains_key(key)
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))
    }

    pub fn mark_state(
        &self,
        tx_type: TransactionType,
        client_id: u16,
        transaction_id: u32,
    ) -> Result<(), ApplicationError> {
        let key = Self::index_key(client_id, transaction_id);
        let _ = self
            .get_tree(tx_type)?
            .insert(key, &[] as &[u8])
            .map_err(|e| ApplicationError::LedgerError(e.to_string()))?;
        Ok(())
    }

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
