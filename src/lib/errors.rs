use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("request failed to `{path}`")]
    Reqwest {
        #[source]
        e: reqwest::Error,
        path: &'static str,
    },

    #[error("IO error")]
    ReqwestIo {
        #[from]
        source: reqwest::Error
    },

    #[error("Http parsing error: {message}")]
    HttpParsingError {
        message: &'static str
    },

    #[error(transparent)]
    UnexpectedError(#[from] Box<dyn std::error::Error>),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;