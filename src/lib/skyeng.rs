use scraper::Html;

use thiserror::Error;
use crate::lib::skyeng::Error::HttpParsingError;

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

async fn get_csrf() -> Result<String> {
    let client = reqwest::Client::new();
    let path = "https://id.skyeng.ru/login";
    let res = client.get(path).send().await.map_err(|e| Error::Reqwest { e, path })?;

    let response = res.text().await?;
    let doc = Html::parse_document(&response);
    let selector = scraper::Selector::parse("input[name=csrfToken]")
        .map_err(|_| HttpParsingError { message: "Unable to parse the page with csrf token" })?;
    let csrf = doc
        .select(&selector)
        .next()
        .ok_or("Unable to find csrf element on the page")
        .and_then(|v|
            v
                .value()
                .attr("value")
                .ok_or("Unable to find value of csrf token element")
        );

    csrf.map(|v| v.to_string())
        .map_err(|e| HttpParsingError { message: e })
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test_csrf() {
        let result = get_csrf().await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
        let csrf = result.unwrap();
        println!("CSRF: {}", csrf);
        assert!(csrf.len() > 0)
    }
}