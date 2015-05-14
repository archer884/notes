#[derive(Debug)]
pub enum CliError {
    Args,
    Io(::std::io::Error),
    Map,
}

impl From<::std::io::Error> for CliError {
    fn from(error: ::std::io::Error) -> CliError {
        CliError::Io(error)
    }
}

impl From<::std::num::ParseIntError> for CliError {
    fn from(_: ::std::num::ParseIntError) -> CliError {
        CliError::Map
    }
}
