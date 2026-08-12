use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    // Increases available funds.
    #[default]
    Deposit,
    // Decreases available funds. Errors on insufficient available funds.
    Withdrawal,
    // Increases held funds, decreases available funds by the same amount. Errors on transaction reference not existing.
    Dispute,
    // Decreases held funds, increase available funds by same amount. Errors on transaction reference not existing.
    Resolve,
    // Decreases available funds. Locks account.
    Chargeback,
}
