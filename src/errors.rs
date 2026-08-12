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
    #[error("Conversion failed: {0}")]
    StructConversionError(String),
}
