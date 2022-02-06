use std::collections::HashMap;
use std::time::UNIX_EPOCH;
use reqwest::Client;
use scraper::Html;
use crate::lib::errors::Error::ServerError;
use serde::Deserialize;

use super::errors::{
    Error,
    Error::HttpParsingError,
    Result,
};

#[derive(Debug)]
pub struct Token {
    value: String,
    expires: u128,
}

#[derive(Debug, Deserialize)]
pub struct WordSet {
    pub meta: Meta,
    pub data: Vec<WordSetData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub total: u32,
    pub current_page: u32,
    pub last_page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordSetData {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordData {
    pub meaning_id: u32,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct Words {
    pub meta: Meta,
    pub data: Vec<WordData>,
}


/// Calls login endpoint and gets csrf token + session cookies
async fn get_csrf(client: &Client) -> Result<String> {
    let path = "https://id.skyeng.ru/login".to_string();
    let res = client.get(&path).send().await.map_err(|e| Error::Reqwest { e, path })?;

    let response = res.text().await?;
    let doc = Html::parse_document(&response);
    let selector = scraper::Selector::parse("input[name=csrfToken]")
        .map_err(|_| HttpParsingError { message: "Unable to parse the page with csrf token" })?;
    let csrf = doc
        .select(&selector)
        .next()
        .ok_or("Unable to find csrf element on the page")
        .and_then(|v| v
            .value()
            .attr("value")
            .ok_or("Unable to find value of csrf token element")
            .map(|v| v.to_string())
        );

    csrf.map_err(|e| HttpParsingError { message: e })
}

fn curr_millis() -> u128 {
    let now = std::time::SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_millis()
}

async fn get_jwt(client: &Client) -> Result<Token> {
    let path = "https://id.skyeng.ru/user-api/v1/auth/jwt".to_string();
    let res = client.post(&path).send().await.map_err(|e| Error::Reqwest { e, path })?;
    let mut cookies = res.cookies();
    cookies.next()
        .ok_or_else(|| ServerError { message: "jwt token was not found in response "})
        .map(|v| Token {
            value: v.value().to_string(),
            expires: v.expires().unwrap()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards").as_millis()
        })
}

async fn login(user: &String, password: &String) -> Result<Token> {
    let client = reqwest::Client::builder()
        .cookie_store(true).build().unwrap();
    let csrf = get_csrf(&client).await?;
    let path = "https://id.skyeng.ru/frame/login-submit".to_string();
    let mut params = HashMap::new();
        params.insert("username", user);
        params.insert("password", password);
        params.insert("csrfToken", &csrf);

    let rs = client.post(&path)
        .form(&params)
        .send().await
        .map_err(|e| Error::Reqwest { e, path })?;

    if rs.status().is_server_error() {
        return Err(Error::ServerError { message: "Bad credentials" });
    }
    if rs.status().is_client_error() {
        return Err(Error::UserError { message: "Bad credentials" });
    }
    get_jwt(&client).await
}

async fn get_word_sets(token: &Token, student_id: u32) -> Result<Vec<WordSetData>> {
    let path = "https://api.words.skyeng.ru/api/for-vimbox/v1/wordsets.json".to_string();
    let page_size = 100;
    let client = reqwest::Client::new();
    let mut current_page = 1;
    let mut word_sets = Vec::new();
    loop {
        let mut params = HashMap::new();
        params.insert("page", &current_page);
        params.insert("pageSize", &page_size);
        params.insert("studentId", &student_id);
        let res = client.get(&path)
            .query(&params)
            .bearer_auth(&token.value)
            .send().await.map_err(|e| Error::Reqwest { e, path: path.clone() })?;
        let response = res.json::<WordSet>().await.map_err(|e| Error::Reqwest { e, path: path.clone() })?;
        word_sets.extend(response.data);
        if response.meta.current_page == response.meta.last_page {
            break;
        }
        current_page += 1;
    }
    Ok(word_sets)
}

pub async fn get_words(token: &Token, student_id: u32) -> Result<Vec<WordData>> {
    let words_sets = get_word_sets(token, student_id).await?;
    let mut words = Vec::new();
    for word_set in words_sets {
        let path = format!("https://api.words.skyeng.ru/api/v1/wordsets/{}/words.json", word_set.id);
        let client = reqwest::Client::new();
        let page_size = "100".to_string();
        let accepted_language = "ru".to_string();
        let mut current_page = 1;
        loop {
            let res = client.get(&path)
                .bearer_auth(&token.value)
                .query(&[
                    ("page", &current_page.to_string()),
                    ("pageSize", &page_size),
                    ("studentId", &student_id.to_string()),
                    ("acceptLanguage", &accepted_language),
                    ("noCache", &curr_millis().to_string()),
                ])
                .send().await.map_err(|e| Error::Reqwest { e, path: path.clone() })?;
            let response = res.json::<Words>().await.map_err(|e| Error::Reqwest { e, path: path.clone() })?;
            words.extend(response.data);
            if response.meta.current_page == response.meta.last_page {
                break;
            }
            current_page += 1;
        }
    }
    Ok(words)
}

#[cfg(test)]
mod test {
    use std::env;
    use super::*;

    #[tokio::test]
    pub async fn test_csrf() {
        let client = reqwest::Client::builder()
            .cookie_store(true).build().unwrap();
        let result = get_csrf(&client).await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
        let csrf = result.unwrap();
        println!("CSRF: {}", csrf);
        assert!(csrf.len() > 0)
    }

    #[tokio::test]
    // Token {
    //   value: "some token",
    //   expires: 1644763320000,
    // }
    pub async fn test_login() {
        let result = login(
            &"red.avtovo@gmail.com".to_string(),
            &env::var("PASS").expect("User password expected")
        ).await;
        println!("result: {:#?}", result);
        assert!(result.unwrap().value.len() > 0);
    }

    #[tokio::test]
    pub async fn test_get_word_sets() {
        let auth_result = login(
            &"red.avtovo@gmail.com".to_string(),
            &env::var("PASS").expect("User password expected")
        ).await;
        let token = auth_result.unwrap();
        let result = get_word_sets(&token,6605911).await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn test_get_words() {
        let auth_result = login(
            &"red.avtovo@gmail.com".to_string(),
            &env::var("PASS").expect("User password expected")
        ).await;
        let token = auth_result.unwrap();
        let result = get_words(&token,6605911).await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
    }
}