use crate::internals::shared::deserialize_u128_fixed;
use crate::internals::transaction_type::TransactionType;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    // Could be missing or invalid id (due to data error)
    #[serde(rename = "tx")]
    pub transaction_id: Option<u32>,
    // Convert float to integer to avoid rounding errors in floating point math, scaled accordingly
    #[serde(deserialize_with = "deserialize_u128_fixed")]
    pub amount: Option<u128>,
}
