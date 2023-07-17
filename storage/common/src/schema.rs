use diesel::prelude::*;

table! {
    contracts (id) {
        id -> Int4,
        uuid -> Varchar,
        state -> Varchar,
        content -> Text,
        key -> Varchar,
    }
}

table! {
    events (id) {
        id -> Int4,
        event_id -> Varchar,
        content -> Text,
        key -> Varchar,
    }
}
