-- Your SQL goes here
CREATE table execution(
    last_update TEXT NOT NULL PRIMARY KEY
);

CREATE table token(
  login TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL,
  expires_at TIMESTAMP NOT NULL
);