use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "toy-payments-engine",
    version,
    about = "Processes a CSV file of transactions"
)]
pub struct Cli {
    #[arg(
        value_name = "Transactions File",
        required = true,
        help = "The CSV file to process"
    )]
    pub file_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use crate::cli::Cli;
    use crate::errors::ApplicationError::CliParse;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_cli_required_argument_provided() {
        let args = vec!["toy-payments-engine", "test_input/cli_arg.csv"];
        let parsed = Cli::try_parse_from(args);
        assert!(parsed.is_ok());
        let cli = parsed.unwrap();
        assert_eq!(cli.file_path, PathBuf::from("test_input/cli_arg.csv"));
    }

    #[test]
    fn test_cli_missing_argument() {
        let args: Vec<&str> = vec!["toy-payments-engine"];
        let parsed = Cli::try_parse_from(args);
        assert!(parsed.is_err());
        let error = parsed.unwrap_err();
        assert!(matches!(error.into(), CliParse(_)));
    }

    #[test]
    fn test_cli_extra_arguments() {
        let input_args = vec![
            "toy-payments-engine",
            "test_input/cli_arg.csv",
            "unexpected_second_file.csv",
        ];
        let parsed = Cli::try_parse_from(input_args);
        assert!(parsed.is_err());
        let error = parsed.unwrap_err();
        assert!(matches!(error.into(), CliParse(_)));
    }
}
