use crate::internals::shared::serialize_u128_fixed;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Default)]
pub struct ClientAccount {
    client_id: u16,
    available: u128,
    held: u128,
    locked: bool,
}

impl ClientAccount {
    pub fn new(client_id: u16, available: u128, held: u128, locked: bool) -> Self {
        Self {
            client_id,
            available,
            held,
            locked,
        }
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

    pub fn deposit(&mut self, amount: u128) -> Result<(), String> {
        self.available += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: u128) -> Result<(), String> {
        if self.available < amount {
            return Err("Insufficient funds".to_string());
        }
        Ok(())
    }

    pub fn dispute(&mut self, amount: u128) -> Result<(), String> {
        if self.available < amount {
            return Err("Funds unavailable for dispute".to_string());
        }
        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    pub fn resolve(&mut self, amount: u128) -> Result<(), String> {
        if self.held < amount {
            return Err("Funds unavailable for resolve".to_string());
        }
        self.held -= amount;
        self.available += amount;
        Ok(())
    }

    pub fn chargeback(&mut self, amount: u128) -> Result<(), String> {
        if self.held < amount {
            return Err("Funds unavailable for chargeback".to_string());
        }
        self.held -= amount;
        self.lock();
        Ok(())
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
            total: u128, // Injected total
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
