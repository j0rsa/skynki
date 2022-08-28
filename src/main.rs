mod lib;
mod schema;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

// This macro from `diesel_migrations` defines an `embedded_migrations` module
// containing a function named `run`. This allows the example to be run and
// tested without any outside setup of the database.
embed_migrations!();

use std::env;
use std::process::exit;
use chrono::Utc;
use lib::{
    skyeng::*,
    anki::*,
};
use crate::lib::db_config::DbConfig;
use crate::lib::repository::{get_last_update, get_token, save_last_update, save_token, save_word, Word};
use diesel_migrations::RunMigrationsError;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let pool = DbConfig::get_pool();
    DbConfig::test_connection(pool.clone()).unwrap();
    let migrations: Result<(),RunMigrationsError> = embedded_migrations::run(&pool.clone().get().unwrap());
    match migrations {
        Ok(_) => {},
        Err(err) => {
            println!("Unable to run migrations: {}!", err);
            exit(1);
        }
    }


    let user = env::var("SKYENG_USERNAME")
        .expect("SKYENG_USERNAME must be set");
    let token: Option<Token> = get_token(&pool, &user)
        .expect("Failed to get last token from db");
    let last_update = get_last_update(&pool)
        .expect("Failed to get last update from db");

    let mut skyeng = Skyeng::new_with_token(
        token,
        user.clone(),
        env::var("SKYENG_PASSWORD")
            .expect("SKYENG_PASSWORD must be set"),
    );

    let callback_pool = pool.clone();
    let login = user.clone();
    skyeng.on_token_update(move |token| {
        println!("Token updated: {:?}", token);
        save_token(&callback_pool, &login, token)
            .expect("Failed to save token to db");
    });

    // let anki = Anki::new(env::var("ANKI_URL").expect("ANKI_URL must be set"));
    // let deck = env::var("ANKI_DECK").unwrap_or("Default".to_string());

    let student = env::var("SKYENG_STUDENT")
        .expect("SKYENG_STUDENT must be set. E.g.: 123")
        .parse::<u32>()
        .expect("SKYENG_STUDENT must be a number");

    let words = skyeng
        .get_words(&student)
        .await
        .expect("Failed to get words from skyeng")
        .created_after(&last_update);
    let data = words.iter().map(|w| w.word.clone()).collect::<Vec<_>>();
    let meanings = skyeng
        .get_meanings(&data)
        .await
        .expect("Failed to get meanings from skyeng");

    // save meanings into anki
    // for note in meanings.iter().map(|m| {
    //     m.to_notes(&deck)
    // }) {
    //     anki.add_note(note)
    //         .await
    //         .expect("Failed to add note to anki");
    // }
    // anki.sync()
    //     .await
    //     .expect("Failed to sync anki");

    // EVENTUALLY
    // store words in DB
    to_word_meanings(&student, &words, &meanings).iter().for_each(|w| {
        save_word(&pool, &w.to_db_word())
            .expect("Failed to save word to db");
    });

    //save last added word timestamp if any to DB
    match words.last_created() {
        Some(w) => save_last_update(&pool, &w).unwrap(),
        _ => {}
    };
}

pub struct WordMeaning{
    student_id: u32,
    word: WordOfSet,
    meaning: Meaning,
}

impl WordMeaning {
    pub fn to_db_word(&self) -> Word {
        Word {
            student_id: self.student_id.into(),
            wordset_id: self.word.wordset.id.into(),
            word_id: self.word.word.meaning_id as i64,
            title: self.word.wordset.title.clone(),
            subtitle: self.word.wordset.subtitle.clone(),
            meaning: self.meaning.text.clone(),
            created_at: self.word.word.created_at.clone(),
            exported_at: Some(Utc::now().naive_utc()),
        }
    }
}

pub fn to_word_meanings(student_id: &u32, words: &Vec<WordOfSet>, meanings: &Vec<Meaning>) -> Vec<WordMeaning> {
    let mut result = Vec::new();
    for word in words {
        for meaning in meanings {
            if &meaning.id == &word.word.meaning_id {
                result.push(WordMeaning {
                    student_id: student_id.clone(),
                    word: word.clone(),
                    meaning: meaning.clone(),
                });
            }
        }
    }
    result
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
