pub mod utils;

/// Type of status code of the server response.
#[derive(Debug)]
pub enum StatusCode {
    OK,
    NotExist,
    NotDirectory,
    NotFile,
    ServerError,
    BadRequest,
}

impl ToString for StatusCode {
    fn to_string(&self) -> String {
        use StatusCode::*;
        match self {
            OK => "OK",
            NotExist => "NOT-EXIST",
            NotDirectory => "NOT-DIRECTORY",
            ServerError => "SERVER-ERROR",
            NotFile => "NOT-FILE",
            BadRequest => "BAD-REQUEST",
        }
        .into()
    }
}
