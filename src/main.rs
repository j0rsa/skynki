mod lib;
mod schema;

#[macro_use]
extern crate diesel;

use std::env;
use lib:: {
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
        env::var("SKYENG_PASSWORD").unwrap()
    );

    let callback_pool = pool.clone();
    let login = user.clone();
    skyeng.on_token_update(move |token| {
        println!("Token updated: {:?}", token);
        save_token(&callback_pool, &login, token).unwrap();
    });

    let anki = Anki::new(env::var("ANKI_URL").unwrap());
    let words = skyeng
        .get_words(env::var("SKYENG_STUDENT").unwrap().parse::<u32>().unwrap())
        .await
        .unwrap()
        .created_after(&last_update);
    let meanings = skyeng.get_meanings(&words).await.unwrap();

    // save meanings into anki
    anki.sync().await.unwrap();

    // EVENTUALLY
    // store meanings in DB

    match words.last_created() {
        Some(w) => save_last_update(&pool, &w).unwrap(),
        _ => {}
    };
    println!("Hello, world!");
}
