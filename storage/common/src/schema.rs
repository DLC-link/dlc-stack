// @generated automatically by Diesel CLI.

diesel::table! {
    contracts (id) {
        id -> Int4,
        uuid -> Varchar,
        state -> Varchar,
        content -> Text,
        key -> Varchar,
    }
}

diesel::table! {
    events (id) {
        id -> Int4,
        event_id -> Varchar,
        content -> Text,
        key -> Varchar,
    }
}
