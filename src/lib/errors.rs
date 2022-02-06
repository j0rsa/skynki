use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("request failed to `{path}`")]
    Reqwest {
        #[source]
        e: reqwest::Error,
        path: String, //&'static str works only with handwritten urls, but not parametrized
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

    #[error("There was a server error: {message}")]
    ServerError {
        message: &'static str
    },

    #[error("There was a user error: {message}")]
    UserError {
        message: &'static str
    },

    #[error(transparent)]
    UnexpectedError(#[from] Box<dyn std::error::Error>),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;