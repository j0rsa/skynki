use std::collections::HashMap;
use std::time::UNIX_EPOCH;
use log::info;
use reqwest::Client;
use scraper::Html;
use crate::lib::errors::Error::{ServerError, UserError};
use serde::Deserialize;

use super::errors::{
    Error,
    Error::HttpParsingError,
    Result,
};

pub(crate) fn curr_millis() -> u128 {
    let now = std::time::SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_millis()
}

#[derive(Debug, Clone)]
pub struct Token {
    value: String,
    expires: u128,
}

trait Expirable {
    fn is_expired(&self) -> bool;
}

impl Expirable for Option<Token> {
    fn is_expired(&self) -> bool {
        match self {
            Some(token) => token.expires < curr_millis(),
            None => true,
        }
    }
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WordSetData {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Clone)]
struct Skyeng {
    client: Client,
    token: Option<Token>,
    user: String,
    password: String,
}

impl Skyeng {
    /// Calls login endpoint and gets csrf token + session cookies
    pub(crate) async fn get_csrf(&self) -> Result<String> {
        let path = "https://id.skyeng.ru/login".to_string();
        let res = self.client.get(&path).send().await.map_err(|e| Error::Reqwest { e, path })?;

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

    async fn get_jwt(&self) -> Result<Token> {
        let path = "https://id.skyeng.ru/user-api/v1/auth/jwt".to_string();
        let res = self.client.post(&path).send().await.map_err(|e| Error::Reqwest { e, path })?;
        let mut cookies = res.cookies();
        cookies.next()
            .ok_or_else(|| ServerError { message: "jwt token was not found in response " })
            .map(|v| Token {
                value: v.value().to_string(),
                expires: v.expires().unwrap()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards").as_millis(),
            })
    }

    pub async fn login(&mut self) -> Result<&mut Self> {
        let csrf = self.get_csrf().await?;
        let path = "https://id.skyeng.ru/frame/login-submit".to_string();
        let mut params = HashMap::new();
        params.insert("username", &self.user);
        params.insert("password", &self.password);
        params.insert("csrfToken", &csrf);
        info!("Logging in user {}", &self.user);
        let rs = self.client.post(&path)
            .form(&params)
            .send().await
            .map_err(|e| Error::Reqwest { e, path })?;

        if rs.status().is_server_error() {
            return Err(Error::ServerError { message: "Bad credentials" });
        }
        if rs.status().is_client_error() {
            return Err(Error::UserError { message: "Bad credentials" });
        }
        let token = self.get_jwt().await?;
        self.token = Some(token);
        Ok(self)
    }

    async fn get_word_sets(&mut self, student_id: u32) -> Result<Vec<WordSetData>> {
        let path = "https://api.words.skyeng.ru/api/for-vimbox/v1/wordsets.json".to_string();
        let page_size = 100;
        let mut current_page = 1;
        let mut word_sets = Vec::new();
        loop {
            info!("Calling {} page {}", path, current_page);
            let mut params = HashMap::new();
            params.insert("page", &current_page);
            params.insert("pageSize", &page_size);
            params.insert("studentId", &student_id);
            let token = self.get_fresh_token().await.ok_or_else(|| UserError { message: "Token is not set. Unable to login" })?;
            let res = self.client.get(&path)
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

    pub async fn get_words(&mut self, student_id: u32) -> Result<Vec<WordData>> {
        let word_sets = self.get_word_sets(student_id).await?;
        let mut words = Vec::new();
        let page_size = "100".to_string();
        let accepted_language = "ru".to_string();
        let curr_time = curr_millis().to_string();
        let token = self.get_fresh_token().await.ok_or_else(|| UserError { message: "Token is not set. Unable to login" })?;
        for word_set in word_sets {
            let path = format!("https://api.words.skyeng.ru/api/v1/wordsets/{}/words.json", word_set.id);
            let mut current_page = 1;
            loop {
                info!("Calling {} page {}", path, current_page);
                let res = self.client.get(&path)
                    .bearer_auth(&token.value)
                    .query(&[
                        ("page", &current_page.to_string()),
                        ("pageSize", &page_size),
                        ("studentId", &student_id.to_string()),
                        ("acceptLanguage", &accepted_language),
                        ("noCache", &curr_time),
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

    #[allow(dead_code)]
    pub fn get_token(&self) -> Option<Token> {
        match self.token.as_ref() {
            Some(token) => Some(token.clone()),
            None => None,
        }
    }

    async fn get_fresh_token(&mut self) -> Option<Token> {
        match if self.token.is_expired() {
            self.login().await
        } else {
            Ok(self)
        }  {
            Ok(v) => v.get_token(),
            _ => None
        }

    }

    fn client() -> Client {
        reqwest::Client::builder().cookie_store(true).build().unwrap()
    }

    pub fn new(user: String, password: String) -> Self {
        Self {
            client: Self::client(),
            token: None,
            user,
            password,
        }
    }

    pub fn new_with_token(token: Option<Token>, user: String, password: String) -> Self {
        Self {
            client: Self::client(),
            token,
            user,
            password,
        }
    }

    async fn get_meanings(&mut self, words: &Vec<WordData>) -> Result<Vec<Meaning>> {
        let path = "https://dictionary.skyeng.ru/api/for-services/v2/meanings".to_string();
        let ids = words.into_iter()
            .map(|w| w.meaning_id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let token = self.get_fresh_token().await.ok_or_else(|| UserError { message: "Token is not set. Unable to login" })?;
        let res = self.client.get(&path)
            .bearer_auth(&token.value)
            .query(&[("ids", &ids)])
            .send().await.map_err(|e| Error::Reqwest { e, path: path.clone() })?;
        let body = String::from_utf8(res.bytes().await.map_err(|e| Error::Reqwest { e, path: path.clone() })?.to_vec()).unwrap();
        serde_json::from_str(&body)
            .map_err(|e| Error::DeserializationError { e, message: body.clone() })
    }
}


#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Meaning {
    pub alternatives: Option<Vec<MeaningAlternative>>,
    pub definition: MeaningDefinition,
    pub examples: Vec<MeaningDefinition>,
    pub id: u64,
    pub images: Vec<MeaningImage>,
    pub sound_url: String,
    pub text: String,
    pub transcription: String,
    pub translation: Translation,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MeaningAlternative {
    pub text: String,
    pub translation: Option<Translation>,
}

#[derive(Debug, Deserialize, Clone)]
struct Translation {
    pub text: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MeaningDefinition {
    pub text: String,
    pub sound_url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct MeaningImage {
    pub url: String,
}

trait NewWords {
    fn created_after(self, date_time: String) -> Vec<WordData>;
    fn last_created(&self) -> Option<String>;
}

impl NewWords for Vec<WordData> {
    fn created_after(self, date_time: String) -> Vec<WordData> {
        self.iter().filter(|w| w.created_at > date_time).cloned().collect()
    }

    fn last_created(&self) -> Option<String> {
        self.iter()
            .map(|w| &w.created_at)
            .max()
            .map(|w| w.to_string())
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use super::*;
    use test_log::test;

    #[tokio::test]
    pub async fn test_csrf() {
        let skyeng = Skyeng::new("".to_string(), "".to_string());
        let result = skyeng.get_csrf().await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
        let csrf = result.unwrap();
        println!("CSRF: {}", csrf);
        assert!(csrf.len() > 0)
    }

    async fn skyeng() -> Skyeng {
        Skyeng::new_with_token(
            None,
            "red.avtovo@gmail.com".to_string(),
            env::var("PASS").expect("User password expected"),
        ).login().await.unwrap().clone()
    }

    #[tokio::test]
    // Token {
    //   value: "some token",
    //   expires: 1644763320000,
    // }
    pub async fn test_login() {
        let mut skyeng = Skyeng::new(
            "red.avtovo@gmail.com".to_string(),
            env::var("PASS").expect("User password expected"),
        );
        let result = skyeng.login().await;
        assert!(result.is_ok());
        let option = &result.unwrap().token;
        println!("result: {:#?}", option);
        assert!(option.is_some());
        let token = option.as_ref().unwrap();
        assert!(token.value.len() > 0);
    }

    #[test(tokio::test)]
    pub async fn test_get_word_sets() {
        let mut skyeng = skyeng().await;
        let result = skyeng.get_word_sets(6605911).await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn test_get_words() {
        let mut skyeng = skyeng().await;
        let result = skyeng.get_words(6605911).await;
        assert!(result.is_ok());
        println!("result: {}", result.unwrap().len());
    }

    #[tokio::test]
    pub async fn test_get_new_words() {
        let mut skyeng = skyeng().await;
        let result = skyeng.get_words(6605911).await.map(
            |words| words.created_after(
                "2022-01-01T00:00:00.000Z".to_string()
            )
        );
        assert!(result.is_ok());
        println!("result: {}", result.unwrap().len());
    }

    #[tokio::test]
    pub async fn test_get_last_created_words() {
        let mut skyeng = skyeng().await;
        let result = skyeng.get_words(6605911).await
            .unwrap()
            .last_created();
        assert!(result.is_some());
        println!("result: {}", result.unwrap());
    }

    #[tokio::test]
    pub async fn test_get_meanings() {
        let mut skyeng = skyeng().await;
        let words = skyeng.get_words(6605911).await
            .unwrap()
            .iter().rev()
            .take(2)
            .map(|v| v.clone())
            .collect::<Vec<WordData>>();
        let result = skyeng.get_meanings(&words).await;
        println!("result: {:#?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().len() >= 2);
    }
}