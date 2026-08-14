use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    #[default]
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
