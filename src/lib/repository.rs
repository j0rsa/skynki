use diesel::pg::PgConnection;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};
use diesel::r2d2::ConnectionManager;
use crate::Token;
use crate::schema::{
    token,
    execution,
    words,
};
use super::errors::Result;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn get_token(pool: &Pool, login: &String) -> Result<Option<Token>> {
    let connection = pool.get()?;
    token::table
        .filter(token::login.eq(login))
        .first::<DbToken>(&connection)
        .optional()
        .map(|token| token.map(|token| token.into()))
        .map_err(|e| e.into())
}

pub fn save_token(pool: &Pool, login: &String, token: &Token) -> Result<()> {
    let connection = pool.get()?;
    let db_token = DbToken::from(login, token);
    diesel::insert_into(token::table)
        .values(&db_token)
        .on_conflict(token::login)
        .do_update()
        .set(&db_token)
        .execute(&connection)?;
    Ok(())
}

#[derive(Insertable, Queryable, AsChangeset, Clone, Debug)]
#[table_name = "token"]
struct DbToken {
    pub login: String,
    pub value: String,
    pub expires_at: chrono::NaiveDateTime,
}

impl Into<Token> for DbToken {
    fn into(self) -> Token {
        let expires = self.expires_at.timestamp_millis() as u128;
        Token::new(self.value, expires)
    }
}

impl DbToken {
    fn from(login: &String, token: &Token) -> DbToken {
        let timestamp_string = token.as_ref().expires.to_string();
        let secs = timestamp_string[..timestamp_string.len() - 6].parse().unwrap();
        let microsecs = timestamp_string[timestamp_string.len() - 6..].parse::<u32>().unwrap();
        DbToken {
            login: login.clone(),
            value: token.as_ref().value.clone(),
            expires_at: chrono::NaiveDateTime::from_timestamp_opt(secs, microsecs * 1000).unwrap(),
        }
    }
}

/// Table is used to keep last run time of the script to filter out the words,
/// which were already processed.
///
/// Could be also calculated from words table by finding max `created_at` column.
#[derive(Insertable, Queryable, AsChangeset, Clone, Debug)]
#[table_name = "execution"]
struct LastUpdate{
    pub last_update: String,
}

impl From<&String> for LastUpdate {
    fn from(last_update: &String) -> Self {
        LastUpdate {
            last_update: last_update.clone(),
        }
    }
}

pub fn get_last_update(pool: &Pool) -> Result<Option<String>> {
    let connection = pool.get()?;
    execution::table
        .select(execution::last_update)
        .first::<String>(&connection)
        .optional()
        .map_err(|e| e.into())
}

pub fn save_last_update(pool: &Pool, last_update: &String) -> Result<()> {
    let connection = pool.get()?;
    let update = LastUpdate::from(last_update);
    diesel::insert_into(execution::table)
        .values(&update)
        .on_conflict(execution::last_update)
        .do_update()
        .set(&update)
        .execute(&connection)?;
    Ok(())
}

#[derive(Insertable, Queryable, Clone, Debug)]
#[table_name="words"]
pub struct Word {
    pub student_id: i64,
    pub wordset_id: i64,
    pub word_id: i64,
    pub title: String,
    pub subtitle: String,
    pub meaning: String,
    pub created_at: String,
    pub exported_at: Option<chrono::NaiveDateTime>,
}

pub fn save_word(pool: &Pool, word: &Word) -> Result<()> {
    let connection = pool.get()?;
    diesel::insert_into(words::table)
        .values(word)
        .on_conflict_do_nothing()
        .execute(&connection)?;
    Ok(())
}