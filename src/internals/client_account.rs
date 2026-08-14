use crate::errors::ApplicationError;
use crate::internals::{
    ledger::Ledger, operation::Operation, shared::serialize_u128_fixed, transaction::Transaction,
    transaction_type::TransactionType,
};
use serde::{Serialize, Serializer};
// Prevent overlap with sled TransactionError
use ApplicationError::TransactionError as TxError;

#[derive(Clone)]
pub struct ClientAccount {
    client_id: u16,
    available: u128,
    held: u128,
    locked: bool,
    ledger: Ledger,
}

impl ClientAccount {
    pub fn new(client_id: u16, ledger: Ledger) -> Self {
        Self {
            client_id,
            available: 0,
            held: 0,
            locked: false,
            ledger,
        }
    }

    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn available(&self) -> u128 {
        self.available
    }

    pub fn held(&self) -> u128 {
        self.held
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn total(&self) -> u128 {
        self.available + self.held
    }

    pub fn deposit(&mut self, tx: Transaction) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Deposit {
            return Err(TxError("Not a deposit".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self
            .ledger
            .is_state(tx.transaction_type, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already deposited".to_string()));
        }
        self.ledger
            .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
        self.available += tx.amount;
        Ok(())
    }

    pub fn withdraw(&mut self, tx: Transaction) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Withdrawal {
            return Err(TxError("Not a withdrawal".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if self
            .ledger
            .is_state(tx.transaction_type, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already withdrawn".to_string()));
        }
        if self.available < tx.amount {
            return Err(TxError("Insufficient funds".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        self.ledger
            .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
        self.available -= tx.amount;
        Ok(())
    }

    pub fn dispute(&mut self, tx: Operation) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Dispute {
            return Err(TxError("Not a dispute".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Dispute, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already disputed".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Resolve, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already resolved".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Chargeback, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already charged back".to_string()));
        }
        let state = self
            .ledger
            .get_transaction(tx.client_id, tx.transaction_id)?;
        match state {
            None => Err(TxError("Failed to retrieve transaction amount".to_string())),
            Some(tr) => {
                let amount = tr.amount.ok_or_else(|| {
                    TxError("Amount not returned with TransactionRecord".to_string())
                })?;
                self.ledger
                    .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
                self.available -= amount;
                self.held += amount;
                Ok(())
            }
        }
    }

    pub fn resolve(&mut self, tx: Operation) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Resolve {
            return Err(TxError("Not a resolution".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Resolve, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already resolved".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Chargeback, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already chargeback".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if !self
            .ledger
            .is_state(TransactionType::Dispute, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError(
                "No active dispute logged for this transaction".to_string(),
            ));
        }
        let state = self
            .ledger
            .get_transaction(tx.client_id, tx.transaction_id)?;
        match state {
            None => Err(TxError("Failed to retrieve transaction amount".to_string())),
            Some(tr) => {
                let amount = tr.amount.ok_or_else(|| {
                    TxError("Amount not returned with TransactionRecord".to_string())
                })?;
                self.ledger
                    .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
                if self.held < amount {
                    return Err(TxError(
                        "Funds unavailable in hold block for resolve".to_string(),
                    ));
                }
                self.held -= amount;
                self.available += amount;
                Ok(())
            }
        }
    }

    pub fn chargeback(&mut self, tx: Operation) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Chargeback {
            return Err(TxError("Not a chargeback".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Resolve, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already resolved".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Chargeback, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already charged back".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if !self
            .ledger
            .is_state(TransactionType::Dispute, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError(
                "No active dispute logged for this transaction".to_string(),
            ));
        }
        let state = self
            .ledger
            .get_transaction(tx.client_id, tx.transaction_id)?;
        match state {
            None => Err(TxError("Failed to retrieve transaction amount".to_string())),
            Some(tr) => {
                let amount = tr.amount.ok_or_else(|| {
                    TxError("Amount not returned with TransactionRecord".to_string())
                })?;
                self.ledger
                    .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
                if self.held < amount {
                    return Err(TxError(
                        "Funds unavailable in hold block for chargeback".to_string(),
                    ));
                }
                self.held -= amount;
                self.lock();
                Ok(())
            }
        }
    }

    fn lock(&mut self) {
        self.locked = true;
    }

    #[allow(dead_code)]
    fn unlock(&mut self) {
        self.locked = false;
    }
}

impl Serialize for ClientAccount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Layout {
            #[serde(rename = "client")]
            client_id: u16,
            #[serde(serialize_with = "serialize_u128_fixed")]
            available: u128,
            #[serde(serialize_with = "serialize_u128_fixed")]
            held: u128,
            #[serde(serialize_with = "serialize_u128_fixed")]
            total: u128,
            locked: bool,
        }
        let shadow = Layout {
            client_id: self.client_id(),
            available: self.available(),
            held: self.held(),
            total: self.total(),
            locked: self.locked(),
        };
        shadow.serialize(serializer)
    }
}
