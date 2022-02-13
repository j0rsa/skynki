table! {
    words (student_id, wordset_id, word_id) {
        student_id -> Int8,
        wordset_id -> Int8,
        title -> Text,
        subtitle -> Text,
        word_id -> Int8,
        meaning -> Text,
        created_at -> Text,
        exported_at -> Nullable<Text>,
    }
}

table! {
    execution (last_update) {
        last_update -> Text,
    }
}

table! {
    token(login) {
        login -> Text,
        value -> Text,
        expires_at -> Timestamp,
    }
}
