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
        #[max_length = 255]
        chain -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    contracts,
    events,
);
