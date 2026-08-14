use crate::errors::ApplicationError;
use crate::internals::{
    Operation, Transaction, TransactionRecord, ledger::Ledger, shared::serialize_u128_fixed,
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
    /// Creates a new zero-balance, unlocked [`ClientAccount`] using `client_id`
    pub fn new(client_id: u16, ledger: Ledger) -> Self {
        Self {
            client_id,
            available: 0,
            held: 0,
            locked: false,
            ledger,
        }
    }

    /// Returns the accounts id
    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    /// Returns available funds, this is funds that have no disputes
    pub fn available(&self) -> u128 {
        self.available
    }

    /// Returns funds that are held due to disputes
    pub fn held(&self) -> u128 {
        self.held
    }

    /// Returns if the account is locked or not
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Returns the total funds of the account
    pub fn total(&self) -> u128 {
        self.available + self.held
    }

    /// Deposits funds into the account, increasing the available balance and marking the transaction
    /// as being processed in the ledger no error condition occur.
    /// It returns a [`ApplicationError::TransactionError`] in the following cases:
    /// - Tx is not a Deposit
    /// - Tx has a client_id different to this account
    /// - Tx has already been processed
    ///
    /// If the account is locked, it can still receive deposits and this is not an
    /// error case.
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
            return Err(TxError("Already processed deposit".to_string()));
        }
        self.ledger.append_transaction(&TransactionRecord {
            transaction_type: tx.transaction_type,
            client_id: tx.client_id,
            transaction_id: Some(tx.transaction_id),
            amount: Some(tx.amount),
        })?;
        self.ledger
            .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
        self.available += tx.amount;
        Ok(())
    }

    /// Withdraws funds from the account, decreasing the available balance and marking the transaction
    /// as being processed in the ledger no error condition occur.
    /// It returns a [`ApplicationError::TransactionError`] in the following cases:
    /// - Tx is not a Withdrawal
    /// - Tx has a client_id different to this account
    /// - Account is locked
    /// - Tx has already been processed
    /// - Account does not have enough funds for the transaction
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
            return Err(TxError("Already processed withdrawal".to_string()));
        }
        self.ledger.append_transaction(&TransactionRecord {
            transaction_type: tx.transaction_type,
            client_id: tx.client_id,
            transaction_id: Some(tx.transaction_id),
            amount: Some(tx.amount),
        })?;
        if self.available < tx.amount {
            return Err(TxError("Insufficient available funds".to_string()));
        }
        self.ledger
            .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
        self.available -= tx.amount;
        Ok(())
    }

    /// Disputes a transaction and marks the operation
    /// as being processed in the ledger when no error condition occurs.
    /// It returns a [`ApplicationError::TransactionError`] in the following cases:
    /// - Tx is not a Dispute
    /// - Tx has a client_id different to this account
    /// - Account is locked
    /// - Tx has already been disputed before (can only be disputed once)
    /// - Tx has already been resolved/chargeback before
    /// - Account does not have enough available funds to be held for dispute
    /// - Deposit cannot be retrieved from storage for [`crate::internals::transaction_record::TransactionRecord`] referenced by operation
    pub fn dispute(&mut self, tx: Operation) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Dispute {
            return Err(TxError("Not a dispute".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Dispute, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed dispute".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Resolve, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed resolved".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Chargeback, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed chargedback".to_string()));
        }
        let state = self
            .ledger
            .get_transaction(tx.client_id, tx.transaction_id)?;
        match state {
            None => Err(TxError("Failed to retrieve transaction amount".to_string())),
            Some(tr) => {
                if tr.transaction_type != TransactionType::Deposit {
                    return Err(TxError(
                        "Only deposits are currently disputable".to_string(),
                    ));
                }
                let amount = tr.amount.ok_or_else(|| {
                    TxError("Amount not returned with TransactionRecord".to_string())
                })?;
                if self.available < amount {
                    return Err(TxError("Insufficient available funds".to_string()));
                }
                self.ledger
                    .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
                self.available -= amount;
                self.held += amount;
                Ok(())
            }
        }
    }

    /// Resolves a disputed transaction and marks the operation
    /// as being processed in the ledger when no error condition occurs.
    /// It returns a [`ApplicationError::TransactionError`] in the following cases:
    /// - Tx is not a Resolve
    /// - Tx has a client_id different to this account
    /// - Account is locked
    /// - Tx has already been resolved/chargeback before
    /// - Tx is not disputed previously
    /// - Account does not have enough held funds to be returned
    /// - Original deposit cannot be retrieved from storage for [`crate::internals::transaction_record::TransactionRecord`] referenced by operation
    pub fn resolve(&mut self, tx: Operation) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Resolve {
            return Err(TxError("Not a resolution".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Resolve, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed resolved".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Chargeback, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed chargeback".to_string()));
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
                // TODO: Depost type check, already done for original dispute though
                let amount = tr.amount.ok_or_else(|| {
                    TxError("Amount not returned with TransactionRecord".to_string())
                })?;
                if self.held < amount {
                    return Err(TxError("Insufficient held funds".to_string()));
                }
                self.ledger
                    .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
                self.held -= amount;
                self.available += amount;
                Ok(())
            }
        }
    }

    /// Chargeback a disputed transaction and marks the operation
    /// as being processed in the ledger when no error condition occurs.
    /// It returns a [`ApplicationError::TransactionError`] in the following cases:
    /// - Tx is not a Chargeback
    /// - Tx has a client_id different to this account
    /// - Account is locked
    /// - Tx has already been resolved/chargeback before
    /// - Tx is not disputed previously
    /// - Account does not have enough held funds to be returned
    /// - Original deposit cannot be retrieved from storage for [`crate::internals::transaction_record::TransactionRecord`] referenced by operation
    pub fn chargeback(&mut self, tx: Operation) -> Result<(), ApplicationError> {
        if tx.transaction_type != TransactionType::Chargeback {
            return Err(TxError("Not a chargeback".to_string()));
        }
        if tx.client_id != self.client_id() {
            return Err(TxError("Not for this account".to_string()));
        }
        if self.locked {
            return Err(TxError("Account locked".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Resolve, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed resolved".to_string()));
        }
        if self
            .ledger
            .is_state(TransactionType::Chargeback, tx.client_id, tx.transaction_id)?
        {
            return Err(TxError("Already processed chargeback".to_string()));
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
                // TODO: Deposit type check, already done for original dispute though
                let amount = tr.amount.ok_or_else(|| {
                    TxError("Amount not returned with TransactionRecord".to_string())
                })?;
                if self.held < amount {
                    return Err(TxError(
                        "Funds unavailable in hold block for chargeback".to_string(),
                    ));
                }
                self.ledger
                    .mark_state(tx.transaction_type, tx.client_id, tx.transaction_id)?;
                self.held -= amount;
                self.lock();
                // TODO: This probably should be stored as a TransactionRecord in Ledger as well, especially to produce an audit trail later
                Ok(())
            }
        }
    }

    /// Locks the account, only deposits can be received into the account while locked
    fn lock(&mut self) {
        self.locked = true;
    }

    /// Unlocks the account, currently unused
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
        // Serialize to fields: client, available, held, total, locked. Total is an injected field
        // since it is a purely calculated field in the account itself.
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

#[cfg(test)]
mod tests {
    use crate::internals::{
        ClientAccount, Ledger, Operation, Transaction, TransactionRecord, TransactionType,
    };
    use rstest::rstest;

    fn ledger() -> Ledger {
        Ledger::new().unwrap()
    }
    fn client(client_id: u16, ledger: Ledger) -> ClientAccount {
        ClientAccount::new(client_id, ledger)
    }

    fn record(
        tx_type: TransactionType,
        client_id: u16,
        transaction_id: u32,
        amount: u128,
    ) -> TransactionRecord {
        TransactionRecord {
            transaction_type: tx_type,
            client_id,
            transaction_id: Some(transaction_id),
            amount: Some(amount),
        }
    }

    fn op(transaction_type: TransactionType, client_id: u16, transaction_id: u32) -> Operation {
        Operation {
            transaction_type,
            client_id,
            transaction_id,
        }
    }

    #[test]
    fn deposit_increases_available_and_total() {
        let ledger = Ledger::new().unwrap();
        let mut client = ClientAccount::new(1, ledger);

        client
            .deposit(Transaction::new(record(TransactionType::Deposit, 1, 1, 1)).unwrap())
            .unwrap();

        assert_eq!(client.available(), 1);
        assert_eq!(client.held(), 0);
        assert_eq!(client.total(), 1);
    }

    #[test]
    fn withdraw_decreases_available() {
        let ledger = Ledger::new().unwrap();
        let mut client = ClientAccount::new(1, ledger);
        client
            .deposit(Transaction::new(record(TransactionType::Deposit, 1, 1, 2)).unwrap())
            .unwrap();

        client
            .withdraw(Transaction::new(record(TransactionType::Withdrawal, 1, 2, 1)).unwrap())
            .unwrap();
        assert_eq!(client.available(), 1);
    }

    #[test]
    fn withdraw_rejects_insufficient_funds() {
        let ledger = Ledger::new().unwrap();
        let mut client = ClientAccount::new(1, ledger);
        client
            .deposit(Transaction::new(record(TransactionType::Deposit, 1, 1, 2)).unwrap())
            .unwrap();

        assert!(
            client
                .withdraw(Transaction::new(record(TransactionType::Withdrawal, 1, 2, 3)).unwrap())
                .is_err()
        );
        assert_eq!(client.available(), 2);
    }

    #[test]
    fn dispute_moves_available_to_held() {
        let ledger = Ledger::new().unwrap();
        let record = record(TransactionType::Deposit, 1, 1, 1);
        ledger.append_transaction(&record).unwrap();
        let mut client = ClientAccount::new(1, ledger);
        client.deposit(Transaction::new(record).unwrap()).unwrap();
        client.dispute(op(TransactionType::Dispute, 1, 1)).unwrap();
        assert_eq!(client.available(), 0);
        assert_eq!(client.held(), 1);
        assert_eq!(client.total(), 1);
    }
    #[test]
    fn resolve_moves_held_back_to_available() {
        let ledger = Ledger::new().unwrap();
        let record = record(TransactionType::Deposit, 1, 1, 1);
        ledger.append_transaction(&record).unwrap();
        let mut client = ClientAccount::new(1, ledger);
        client.deposit(Transaction::new(record).unwrap()).unwrap();
        client.dispute(op(TransactionType::Dispute, 1, 1)).unwrap();
        client.resolve(op(TransactionType::Resolve, 1, 1)).unwrap();
        assert_eq!(client.available(), 1);
        assert_eq!(client.held(), 0);
        assert!(!client.locked());
    }

    #[test]
    fn chargeback_locks_the_account_and_reduces_held_funds() {
        let ledger = Ledger::new().unwrap();
        let record = record(TransactionType::Deposit, 1, 1, 1);
        ledger.append_transaction(&record).unwrap();
        let mut client = ClientAccount::new(1, ledger);
        client.deposit(Transaction::new(record).unwrap()).unwrap();
        client.dispute(op(TransactionType::Dispute, 1, 1)).unwrap();
        client
            .chargeback(op(TransactionType::Chargeback, 1, 1))
            .unwrap();
        assert_eq!(client.available(), 0);
        assert_eq!(client.held(), 0);
        assert_eq!(client.total(), 0);
        assert!(client.locked());
    }

    #[rstest]
    #[case(TransactionType::Deposit)]
    #[case(TransactionType::Withdrawal)]
    #[case(TransactionType::Dispute)]
    #[case(TransactionType::Resolve)]
    #[case(TransactionType::Chargeback)]
    fn rejects_wrong_transaction_type(#[case] input: TransactionType) {
        let mut client = client(1, ledger());
        match input {
            TransactionType::Deposit => {
                assert!(
                    client
                        .deposit(
                            Transaction::new(record(TransactionType::Withdrawal, 1, 1, 0)).unwrap()
                        )
                        .unwrap_err()
                        .to_string()
                        .contains("Not a deposit")
                );
            }
            TransactionType::Withdrawal => {
                assert!(
                    client
                        .withdraw(
                            Transaction::new(record(TransactionType::Deposit, 1, 1, 0)).unwrap()
                        )
                        .unwrap_err()
                        .to_string()
                        .contains("Not a withdrawal")
                );
            }
            TransactionType::Dispute => {
                assert!(
                    client
                        .dispute(op(TransactionType::Deposit, 1, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Not a dispute")
                );
            }
            TransactionType::Resolve => {
                assert!(
                    client
                        .resolve(op(TransactionType::Deposit, 1, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Not a resolution")
                );
            }
            TransactionType::Chargeback => {
                assert!(
                    client
                        .chargeback(op(TransactionType::Deposit, 1, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Not a chargeback")
                );
            }
        }
    }

    #[rstest]
    #[case(TransactionType::Deposit)]
    #[case(TransactionType::Withdrawal)]
    #[case(TransactionType::Dispute)]
    #[case(TransactionType::Resolve)]
    #[case(TransactionType::Chargeback)]
    fn rejects_wrong_client_id(#[case] input: TransactionType) {
        let mut client = client(1, ledger());
        match input {
            TransactionType::Deposit => {
                assert!(
                    client
                        .deposit(
                            Transaction::new(record(TransactionType::Deposit, 2, 1, 0)).unwrap()
                        )
                        .unwrap_err()
                        .to_string()
                        .contains("Not for this account")
                );
            }
            TransactionType::Withdrawal => {
                assert!(
                    client
                        .withdraw(
                            Transaction::new(record(TransactionType::Withdrawal, 2, 1, 0)).unwrap()
                        )
                        .unwrap_err()
                        .to_string()
                        .contains("Not for this account")
                );
            }
            TransactionType::Dispute => {
                assert!(
                    client
                        .dispute(op(TransactionType::Dispute, 2, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Not for this account")
                );
            }
            TransactionType::Resolve => {
                assert!(
                    client
                        .resolve(op(TransactionType::Resolve, 2, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Not for this account")
                );
            }
            TransactionType::Chargeback => {
                assert!(
                    client
                        .chargeback(op(TransactionType::Chargeback, 2, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Not for this account")
                );
            }
        }
    }

    #[rstest]
    #[case(TransactionType::Deposit)]
    #[case(TransactionType::Withdrawal)]
    #[case(TransactionType::Dispute)]
    #[case(TransactionType::Resolve)]
    #[case(TransactionType::Chargeback)]
    fn rejects_when_locked(#[case] input: TransactionType) {
        let mut client = client(1, ledger());
        client.lock();
        match input {
            TransactionType::Deposit => {
                assert!(
                    !client
                        .deposit(
                            Transaction::new(record(TransactionType::Deposit, 1, 1, 0)).unwrap()
                        )
                        .is_err()
                );
            }
            TransactionType::Withdrawal => {
                assert!(
                    client
                        .withdraw(
                            Transaction::new(record(TransactionType::Withdrawal, 1, 1, 0)).unwrap()
                        )
                        .unwrap_err()
                        .to_string()
                        .contains("Account locked")
                );
            }
            TransactionType::Dispute => {
                assert!(
                    client
                        .dispute(op(TransactionType::Dispute, 1, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Account locked")
                );
            }
            TransactionType::Resolve => {
                assert!(
                    client
                        .resolve(op(TransactionType::Resolve, 1, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Account locked")
                );
            }
            TransactionType::Chargeback => {
                assert!(
                    client
                        .chargeback(op(TransactionType::Chargeback, 1, 1))
                        .unwrap_err()
                        .to_string()
                        .contains("Account locked")
                );
            }
        }
    }

    #[rstest]
    #[case(TransactionType::Deposit)]
    #[case(TransactionType::Withdrawal)]
    #[case(TransactionType::Dispute)]
    #[case(TransactionType::Resolve)]
    #[case(TransactionType::Chargeback)]
    fn rejects_already_processed(#[case] input: TransactionType) {
        let mut client = client(1, ledger());
        match input {
            TransactionType::Deposit => {
                let record = Transaction::new(record(TransactionType::Deposit, 1, 2, 0)).unwrap();
                client.deposit(record.clone()).unwrap();
                assert!(
                    client
                        .deposit(record)
                        .unwrap_err()
                        .to_string()
                        .contains("Already processed")
                );
            }
            TransactionType::Withdrawal => {
                let tx_record =
                    Transaction::new(record(TransactionType::Withdrawal, 1, 2, 10)).unwrap();
                client
                    .deposit(Transaction::new(record(TransactionType::Deposit, 1, 1, 20)).unwrap())
                    .unwrap();
                client.withdraw(tx_record.clone()).unwrap();
                assert!(
                    client
                        .withdraw(tx_record)
                        .unwrap_err()
                        .to_string()
                        .contains("Already processed")
                );
            }
            TransactionType::Dispute => {
                let tx = record(TransactionType::Deposit, 1, 1, 0);
                client.ledger.append_transaction(&tx).unwrap();
                client.deposit(Transaction::new(tx).unwrap()).unwrap();
                let tx_record = op(TransactionType::Dispute, 1, 1);
                client.dispute(tx_record.clone()).unwrap();
                assert!(
                    client
                        .dispute(tx_record)
                        .unwrap_err()
                        .to_string()
                        .contains("Already processed")
                );
                // TODO: Extend to cover all edge cases.
            }
            TransactionType::Resolve => {
                let tx = record(TransactionType::Deposit, 1, 1, 0);
                client.ledger.append_transaction(&tx).unwrap();
                client.deposit(Transaction::new(tx).unwrap()).unwrap();
                let mut tx_record = op(TransactionType::Dispute, 1, 1);
                client.dispute(tx_record.clone()).unwrap();
                tx_record.transaction_type = TransactionType::Resolve;
                client.resolve(tx_record.clone()).unwrap();
                assert!(
                    client
                        .resolve(tx_record)
                        .unwrap_err()
                        .to_string()
                        .contains("Already processed")
                );
                // TODO: Extend to cover all edge cases.
            }
            TransactionType::Chargeback => {
                let tx = record(TransactionType::Deposit, 1, 1, 0);
                client.ledger.append_transaction(&tx).unwrap();
                client.deposit(Transaction::new(tx).unwrap()).unwrap();
                let mut tx_record = op(TransactionType::Dispute, 1, 1);
                client.dispute(tx_record.clone()).unwrap();
                tx_record.transaction_type = TransactionType::Chargeback;
                client.chargeback(tx_record.clone()).unwrap();
                client.unlock();
                assert!(
                    client
                        .chargeback(tx_record)
                        .unwrap_err()
                        .to_string()
                        .contains("Already processed")
                );
                // TODO: Extend to cover all edge cases.
            }
        }
    }

    #[rstest]
    #[case(TransactionType::Resolve)]
    #[case(TransactionType::Chargeback)]
    fn rejects_when_not_disputed_already(#[case] input: TransactionType) {
        let mut client = client(1, ledger());
        match input {
            TransactionType::Resolve => {
                let tx = record(TransactionType::Deposit, 1, 1, 0);
                client.ledger.append_transaction(&tx).unwrap();
                client.deposit(Transaction::new(tx).unwrap()).unwrap();
                let tx_record = op(TransactionType::Resolve, 1, 1);
                assert!(
                    client
                        .resolve(tx_record)
                        .unwrap_err()
                        .to_string()
                        .contains("No active dispute")
                );
            }
            TransactionType::Chargeback => {
                let tx = record(TransactionType::Deposit, 1, 1, 0);
                client.ledger.append_transaction(&tx).unwrap();
                client.deposit(Transaction::new(tx).unwrap()).unwrap();
                let tx_record = op(TransactionType::Chargeback, 1, 1);
                assert!(
                    client
                        .chargeback(tx_record)
                        .unwrap_err()
                        .to_string()
                        .contains("No active dispute")
                );
            }
            _ => {
                unreachable!()
            }
        }
    }
}
