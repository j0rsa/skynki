use super::errors::{Error, Result};
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Action {
    pub version: String,
    pub action: String,
    pub data: Option<serde_json::Value>,
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
            data: None,
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
