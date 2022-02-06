use reqwest::header::HeaderValue;
use scraper::Html;

use super::errors::{
    Error,
    Error::HttpParsingError,
    Result,
};

#[derive(Debug)]
struct SessionCsrf {
    token: String,
    session_info: Vec<String>
}

#[derive(Debug)]
struct Cookie {
    name: String,
    value: String,
}

#[derive(Debug)]
struct Token {
    value: String,
    expires: u64,
}

async fn get_csrf() -> Result<SessionCsrf> {
    let client = reqwest::Client::new();
    let path = "https://id.skyeng.ru/login";
    let res = client.get(path).send().await.map_err(|e| Error::Reqwest { e, path })?;

    let headers = res.headers().clone();
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

    let cookies: Vec<String> = headers.get_all("set-cookie")
        .iter()
        .map(|v|
            String::from_utf8(
                v.as_bytes()
                    .into()
            ).unwrap())
        .collect();

    csrf.map(|v| SessionCsrf{ token: v.to_string(), session_info: cookies } )
        .map_err(|e| HttpParsingError { message: e })
}

async fn login(user: &'static str, password: &'static str) -> Result<Token> {
    Err(Error::HttpParsingError { message : "" })
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
        println!("CSRF: {}", csrf.token);
        assert!(csrf.token.len() > 0)
    }
}