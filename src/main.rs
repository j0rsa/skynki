mod lib;
mod schema;

#[macro_use]
extern crate diesel;

use std::env;
use lib::{
    skyeng::*,
    anki::*,
};
use crate::lib::db_config::DbConfig;
use crate::lib::repository::{get_last_update, get_token, save_last_update, save_token};

async fn main() {
    env_logger::init();

    let pool = DbConfig::get_pool();
    DbConfig::test_connection(pool.clone()).unwrap();

    let user = env::var("SKYENG_USERNAME").unwrap();
    let token: Option<Token> = get_token(&pool, &user).unwrap();
    let last_update = get_last_update(&pool).unwrap();

    let mut skyeng = Skyeng::new_with_token(
        token,
        user.clone(),
        env::var("SKYENG_PASSWORD").unwrap(),
    );

    let callback_pool = pool.clone();
    let login = user.clone();
    skyeng.on_token_update(move |token| {
        println!("Token updated: {:?}", token);
        save_token(&callback_pool, &login, token).unwrap();
    });

    let anki = Anki::new(env::var("ANKI_URL").unwrap());
    let deck = env::var("ANKI_DECK").unwrap_or("Default".to_string());

    let words = skyeng
        .get_words(env::var("SKYENG_STUDENT").unwrap().parse::<u32>().unwrap())
        .await
        .unwrap()
        .created_after(&last_update);
    let meanings = skyeng.get_meanings(&words).await.unwrap();

    // save meanings into anki
    for note in meanings.iter().map(|m| {
        m.to_notes(&deck)
    }) {
        anki.add_note(note).await.unwrap();
    }
    anki.sync().await.unwrap();

    // EVENTUALLY
    // store meanings in DB

    match words.last_created() {
        Some(w) => save_last_update(&pool, &w).unwrap(),
        _ => {}
    };
}

trait ToMaskedText {
    fn to_masked_text(&self) -> String;
}

impl ToMaskedText for Meaning {
    fn to_masked_text(&self) -> String {
        format!("{} {}", self.text, self.translation.text)
    }
}

trait ToAttachment {
    fn to_attachment(&self, fields: Vec<String>) -> Attachment;
}

impl ToAttachment for String {
    fn to_attachment(&self, fields: Vec<String>) -> Attachment  {
        let filename = if self.contains("?") {
            format!("{}.mp3",self.split("=").last().unwrap())
        } else {
            self.split("/").last().unwrap().to_string()
        };

        Attachment {
            url: self.clone(),
            filename,
            fields
        }
    }
}

trait AnkiPersistence {
    fn to_notes(&self, deck_name: &String) -> Note;
}

impl AnkiPersistence for Meaning {
    fn to_notes(&self, deck_name: &String) -> Note {
        let masked_text = self.to_masked_text();
        let audio = self.sound_url.to_attachment(vec![
            "Extra".to_string()
        ]);

        Note {
            deck_name: deck_name.clone(),
            model_name: "Cloze".to_string(),
            fields: Fields {
                text: masked_text
            },
            options: Options {
                allow_duplicate: false,
                duplicate_scope: "deck".to_string(),
                duplicate_scope_options: Some(DuplicateScopeOptions {
                    deck_name: "Default".to_string(),
                    check_children: false,
                    check_all_models: false
                })
            },
            tags: vec![
                "skyeng".to_string(),
            ],
            audio: vec![audio],
            picture: vec![
                Attachment {
                    url: "".to_string(),
                    filename: "".to_string(),
                    fields: vec![
                        "Extra".to_string(),
                    ]
                }
            ]
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_url_picture_parse() {
        let url = "https://cdn-user77752.skyeng.ru/resized-images/200x150/png/50/5a677c4b4a356e7a4a3fc243deb73676.png".to_string();
        let attachment = url.to_attachment(vec![
            "Extra".to_string()
        ]);
        assert_eq!(attachment.url, url);
        assert_eq!(attachment.filename, "5a677c4b4a356e7a4a3fc243deb73676.png".to_string());
        assert_eq!(attachment.fields, vec![
            "Extra".to_string()
        ]);
    }

    #[test]
    fn test_url_sound_parse() {
        let url = "https://d2fmfepycn0xw0.cloudfront.net?gender=female&accent=american&text=deter".to_string();
        let attachment = url.to_attachment(vec![
            "Extra".to_string()
        ]);
        assert_eq!(attachment.url, url);
        assert_eq!(attachment.filename, "deter.mp3".to_string());
        assert_eq!(attachment.fields, vec![
            "Extra".to_string()
        ]);
    }
}
