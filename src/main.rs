mod lib;

use std::env;
use lib:: {
    skyeng::*,
    anki::*,
};

async fn main() {
    let token: Option<Token> = None; // get from DB
    let mut skyeng = Skyeng::new_with_token(
        token,
        env::var("SKYENG_USERNAME").unwrap(),
        env::var("SKYENG_PASSWORD").unwrap()
    );

    skyeng.on_token_update(|token| {
        println!("Token updated: {:?}", token);
    });
    let anki = Anki::new(env::var("ANKI_URL").unwrap());
    let last_date = "".to_string(); // get from DB
    let words = skyeng
        .get_words(env::var("SKYENG_STUDENT").unwrap().parse::<u32>().unwrap())
        .await
        .unwrap()
        .created_after(&last_date);
    let meanings = skyeng.get_meanings(&words).await.unwrap();

    // save meanings into anki
    anki.sync().await.unwrap();

    // EVENTUALLY
    // store meanings in DB

    match words.last_created() {
        Some(w) => {w;},//save in db
        _ => {}
    };
    println!("Hello, world!");
}
