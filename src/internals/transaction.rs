use crate::{
    errors::{ApplicationError, ApplicationError::StructConversionError},
    internals::{TransactionRecord, TransactionType},
};

#[derive(Debug, Default, Clone)]
pub struct Transaction {
    pub transaction_type: TransactionType,
    pub client_id: u16,
    pub transaction_id: u32,
    pub amount: u128,
}

impl Transaction {
    /// Creates a new transaction from a TransactionRecord
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

#[cfg(test)]
mod tests {
    use crate::{
        errors::ApplicationError,
        internals::{Transaction, TransactionRecord, TransactionType},
    };

    fn record(
        transaction_type: TransactionType,
        client_id: u16,
        transaction_id: Option<u32>,
        amount: Option<u128>,
    ) -> TransactionRecord {
        TransactionRecord {
            transaction_type,
            client_id,
            transaction_id,
            amount,
        }
    }

    #[test]
    fn new_succeeds_with_id_and_amount() {
        let tx = Transaction::new(record(TransactionType::Deposit, 2, Some(1), Some(1))).unwrap();
        assert_eq!(tx.transaction_id, 1);
        assert_eq!(tx.amount, 1);
        assert_eq!(tx.client_id, 2);
        assert_eq!(tx.transaction_type, TransactionType::Deposit);
    }

    #[test]
    fn new_fails_without_transaction_id() {
        let err = Transaction::new(record(TransactionType::Deposit, 1, None, Some(1))).unwrap_err();
        assert!(matches!(err, ApplicationError::StructConversionError(_)));
    }

    #[test]
    fn new_fails_without_amount() {
        let err = Transaction::new(record(TransactionType::Deposit, 1, Some(1), None)).unwrap_err();
        assert!(matches!(err, ApplicationError::StructConversionError(_)));
    }
}
