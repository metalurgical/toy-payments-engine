use crate::errors::ApplicationError;
use log::LevelFilter;
use log4rs::config::Logger;
use log4rs::{
    Config,
    append::file::FileAppender,
    config::{Appender, Root},
};

/// Initialized logging to write to `errors.log`
pub fn init_logging() -> Result<(), ApplicationError> {
    let file_appender = FileAppender::builder().build("errors.log")?;
    let config = Config::builder()
        .appender(Appender::builder().build("file_logger", Box::new(file_appender)))
        // Log trace and above for this application
        .logger(Logger::builder().build("toy_payments_engine", LevelFilter::Trace))
        .build(
            // Ignore all other logging from crates used in the application
            Root::builder()
                .appender("file_logger")
                .build(LevelFilter::Off),
        )
        .map_err(|e| ApplicationError::LoggingInitError(e.to_string()))?;
    log4rs::init_config(config).map_err(|e| ApplicationError::LoggingInitError(e.to_string()))?;
    Ok(())
}
