use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Command-line parsing failed: {0}")]
    CliParse(#[from] clap::Error),
    #[error("File processing failed: {0}")]
    IoFailure(#[from] std::io::Error),
    #[error("Parse csv error: {0}")]
    CSVParse(#[from] csv::Error),
    #[error("Ledger processing failed: {0}")]
    LedgerError(String),
    #[error("Transaction failed: {0}")]
    TransactionError(String),
    #[error("Logger initialization failed: {0}")]
    LoggingInitError(String),
    #[error("Conversion failed: {0}")]
    StructConversionError(String),
    #[error("Join  error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}
