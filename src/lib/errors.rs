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

    #[error("Deserialization error: {e}\n{message}")]
    DeserializationError {
        e: serde_json::Error,
        message: String
    },

    #[error("Db error: {source}")]
    DbError {
        #[from]
        source: r2d2::Error
    },

    #[error("Db error: {source}")]
    DieselError {
        #[from]
        source: diesel::result::Error
    },

    #[error(transparent)]
    UnexpectedError(#[from] Box<dyn std::error::Error>),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;