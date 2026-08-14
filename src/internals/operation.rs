use crate::{
    errors::{ApplicationError, ApplicationError::StructConversionError},
    internals::{TransactionRecord, TransactionType},
};

#[derive(Debug, Default, Clone)]
pub struct Operation {
    pub transaction_type: TransactionType,
    pub client_id: u16,
    pub transaction_id: u32,
}

impl Operation {
    /// Created a new operation from a TransactionRecord
    pub fn new(tx: TransactionRecord) -> Result<Self, ApplicationError> {
        if let Some(id) = tx.transaction_id {
            Ok(Self {
                transaction_type: tx.transaction_type,
                client_id: tx.client_id,
                transaction_id: id,
            })
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
        internals::{Operation, TransactionRecord, TransactionType},
    };

    fn record(tx_id: Option<u32>, client: u16) -> TransactionRecord {
        TransactionRecord {
            transaction_type: TransactionType::Resolve,
            client_id: client,
            transaction_id: tx_id,
            amount: None,
        }
    }

    #[test]
    fn new_succeeds_with_transaction_id() {
        let op = Operation::new(record(Some(2), 1)).unwrap();
        assert_eq!(op.transaction_id, 2);
        assert_eq!(op.client_id, 1);
        assert_eq!(op.transaction_type, TransactionType::Resolve);
    }

    #[test]
    fn new_fails_without_transaction_id() {
        let err = Operation::new(record(None, 1)).unwrap_err();
        assert!(matches!(err, ApplicationError::StructConversionError(_)));
    }
}
