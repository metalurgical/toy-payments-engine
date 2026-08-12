use crate::internals::operation::Operation;
use crate::internals::shared::serialize_u128_fixed;
use crate::internals::transaction::Transaction;
use crate::internals::transaction_type::TransactionType;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Default)]
pub struct ClientAccount {
    client_id: u16,
    available: u128,
    held: u128,
    locked: bool,
    desposits: HashMap<u32, Transaction>,
    withdrawals: HashMap<u32, Transaction>,
    disputed: HashMap<u32, Operation>,
    resolved: HashMap<u32, Operation>,
    chargeback: HashMap<u32, Operation>,
}

impl PartialEq<Self> for ClientAccount {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id
    }
}
impl Eq for ClientAccount {}

impl Hash for ClientAccount {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.client_id.to_be_bytes());
    }
}

impl ClientAccount {
    pub fn new(client_id: u16) -> Self {
        Self {
            client_id,
            available: 0,
            held: 0,
            locked: false,
            desposits: Default::default(),
            withdrawals: Default::default(),
            disputed: Default::default(),
            resolved: Default::default(),
            chargeback: Default::default(),
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

    pub fn deposit(&mut self, tx: Transaction) -> Result<(), String> {
        if tx.transaction_type != TransactionType::Deposit {
            return Err("Not a deposit".to_string());
        }
        if tx.client_id != self.client_id() {
            return Err("Not for this account".to_string());
        }

        if self.desposits.contains_key(&tx.transaction_id) {
            return Err("Transaction already deposited".to_string());
        }
        self.desposits.insert(tx.transaction_id, tx.clone());
        self.available += tx.amount;
        Ok(())
    }

    pub fn withdraw(&mut self, tx: Transaction) -> Result<(), String> {
        if tx.transaction_type != TransactionType::Withdrawal {
            return Err("Not a withdrawal".to_string());
        }
        if tx.client_id != self.client_id() {
            return Err("Not for this account".to_string());
        }
        if self.locked {
            return Err("Account locked".to_string());
        }

        if self.withdrawals.contains_key(&tx.transaction_id) {
            return Err("Already withdrawn".to_string());
        }
        if self.available < tx.amount {
            return Err("Insufficient funds".to_string());
        }
        self.withdrawals.insert(tx.transaction_id, tx.clone());
        self.available -= tx.amount;
        Ok(())
    }

    pub fn dispute(&mut self, tx: Operation) -> Result<(), String> {
        if tx.transaction_type != TransactionType::Dispute {
            return Err("Not a dispute".to_string());
        }
        if tx.client_id != self.client_id() {
            return Err("Not for this account".to_string());
        }
        if self.disputed.contains_key(&tx.transaction_id) {
            return Err("Already disputed".to_string());
        }
        match self.desposits.get(&tx.transaction_id) {
            None => Err("No deposit record found to dispute".to_string()),
            Some(original_deposit) => {
                let amount = original_deposit.amount;
                self.available = self.available.saturating_sub(amount);
                self.held += amount;
                self.disputed.insert(tx.transaction_id, tx.clone());
                Ok(())
            }
        }
    }

    pub fn resolve(&mut self, tx: Operation) -> Result<(), String> {
        if tx.transaction_type != TransactionType::Resolve {
            return Err("Not a resolution".to_string());
        }
        if tx.client_id != self.client_id() {
            return Err("Not for this account".to_string());
        }
        if self.resolved.contains_key(&tx.transaction_id) {
            return Err("Already resolved".to_string());
        }
        match self.disputed.get(&tx.transaction_id) {
            None => Err("No active dispute logged for this transaction".to_string()),
            Some(_) => {
                let original_deposit = self
                    .desposits
                    .get(&tx.transaction_id)
                    .ok_or_else(|| "Original deposit missing during resolution".to_string())?;
                let amount = original_deposit.amount;
                if self.held < amount {
                    return Err("Funds unavailable in hold block for resolve".to_string());
                }
                self.resolved.insert(tx.transaction_id, tx.clone());
                self.held -= amount;
                self.available += amount;
                Ok(())
            }
        }
    }

    pub fn chargeback(&mut self, tx: Operation) -> Result<(), String> {
        if tx.transaction_type != TransactionType::Chargeback {
            return Err("Not a chargeback".to_string());
        }
        if tx.client_id != self.client_id() {
            return Err("Not for this account".to_string());
        }
        if self.chargeback.contains_key(&tx.transaction_id) {
            return Err("Already charged back".to_string());
        }
        if self.locked {
            return Err("Account locked".to_string());
        }

        match self.disputed.get(&tx.transaction_id) {
            None => Err("No active dispute logged for this transaction".to_string()),
            Some(_) => {
                let original_deposit = self
                    .desposits
                    .get(&tx.transaction_id)
                    .ok_or_else(|| "Original deposit missing during chargeback".to_string())?;
                let amount = original_deposit.amount;

                if self.held < amount {
                    return Err("Funds unavailable in hold block for chargeback".to_string());
                }

                self.chargeback.insert(tx.transaction_id, tx.clone());
                self.held -= amount;
                self.lock();
                Ok(())
            }
        }
    }

    fn lock(&mut self) {
        self.locked = true;
    }
}

impl Serialize for ClientAccount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Layout {
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
            client_id: self.client_id,
            available: self.available,
            held: self.held,
            total: self.total(),
            locked: self.locked,
        };

        shadow.serialize(serializer)
    }
}
