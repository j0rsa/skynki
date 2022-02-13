use super::errors::{Error, Result};
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Action {
    pub version: String,
    pub action: String,
    pub params: Option<Note>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub deck_name: String,
    pub model_name: String,
    pub fields: Fields,
    pub options: Options,
    pub tags: Vec<String>,
    pub audio: Vec<Attachment>,
    pub picture: Vec<Attachment>,
}

#[derive(Serialize, Debug)]
pub struct Fields {
    #[serde(rename = "Text")]
    pub text: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Options{
    pub allow_duplicate: bool, //false
    pub duplicate_scope: String, //deck
    pub duplicate_scope_options: Option<DuplicateScopeOptions>,

}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScopeOptions {
    pub deck_name: String, //"Default"
    pub check_children: bool, //false
    pub check_all_models: bool, //false
}

#[derive(Serialize, Debug)]
pub struct Attachment {
    pub url: String,
    pub filename: String,
    pub fields: Vec<String>, //[Extra]
}

pub struct Anki {
    client: reqwest::Client,
    url: String,
}

impl Anki {
    pub fn new(url: String) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            url,
        }
    }
    pub async fn sync(&self) -> Result<()> {
        let data = Action {
            version: "6".to_string(),
            action: "sync".to_string(),
            params: None,
        };
        let path = self.url.clone();
        self.client.post(&path)
            .json(&data)
            .send().await
            .map(|_| ())
            .map_err(|e| Error::Reqwest { e, path })
    }

    pub async fn add_note(&self, note: Note) -> Result<()> {
        let data = Action {
            version: "6".to_string(),
            action: "addNote".to_string(),
            params: Some(note),
        };
        let path = self.url.clone();
        self.client.post(&path)
            .json(&data)
            .send().await
            .map(|_| ())
            .map_err(|e| Error::Reqwest { e, path })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_anki_sync() {
        let anki = Anki::new("http://10.43.149.198".to_string());
        anki.sync().await.unwrap();
    }
}
