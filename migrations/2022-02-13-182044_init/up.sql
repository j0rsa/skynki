-- Your SQL goes here
CREATE TABLE execution(
    last_update TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE token(
  login TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL,
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE words(
    student_id INTEGER NOT NULL,
    wordset_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    subtitle TEXT NOT NULL,
    word_id INTEGER NOT NULL,
    meaning TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TIMESTAMP,

    PRIMARY KEY(student_id, wordset_id, word_id)
);