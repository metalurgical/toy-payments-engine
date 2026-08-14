use crate::{
    errors::{ApplicationError, ApplicationError::StructConversionError},
    internals::{transaction_record::TransactionRecord, transaction_type::TransactionType},
};

#[derive(Debug, Default, Clone)]
pub struct Transaction {
    pub transaction_type: TransactionType,
    pub client_id: u16,
    pub transaction_id: u32,
    pub amount: u128,
}

impl Transaction {
    pub fn new(tx: TransactionRecord) -> Result<Self, ApplicationError> {
        if let Some(id) = tx.transaction_id {
            if let Some(amount) = tx.amount {
                Ok(Self {
                    transaction_type: tx.transaction_type,
                    client_id: tx.client_id,
                    transaction_id: id,
                    amount,
                })
            } else {
                Err(StructConversionError(String::from("Amount is None")))
            }
        } else {
            Err(StructConversionError(String::from(
                "Transaction Id is None",
            )))
        }
    }
}
