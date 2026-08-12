use crate::errors::ApplicationError;
use crate::errors::ApplicationError::StructConversionError;
use crate::internals::transaction_record::TransactionRecord;
use crate::internals::transaction_type::TransactionType;

#[derive(Debug, Default, Clone)]
pub struct Operation {
    pub transaction_type: TransactionType,
    pub client_id: u16,
    pub transaction_id: u32,
}

impl Operation {
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
