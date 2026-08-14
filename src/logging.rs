use crate::errors::ApplicationError;
use log::LevelFilter;
use log4rs::config::Logger;
use log4rs::{
    Config,
    append::file::FileAppender,
    config::{Appender, Root},
};

pub fn init_logging() -> Result<(), ApplicationError> {
    let file_appender = FileAppender::builder().build("errors.log").unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("file_logger", Box::new(file_appender)))
        .logger(Logger::builder().build("toy_payments_engine", LevelFilter::Trace))
        .build(
            Root::builder()
                .appender("file_logger")
                .build(LevelFilter::Off),
        )
        .map_err(|e| ApplicationError::LoggingInitError(e.to_string()))?;

    log4rs::init_config(config).map_err(|e| ApplicationError::LoggingInitError(e.to_string()))?;
    Ok(())
}
