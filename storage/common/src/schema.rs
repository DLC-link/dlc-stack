use diesel::prelude::*;

table! {
    contracts (id) {
        id -> Int4,
        uuid -> Varchar,
        state -> Varchar,
        content -> Text,
    }
}

table! {
    events (id) {
        id -> Int4,
        event_id -> Varchar,
        content -> Text,
    }
}
