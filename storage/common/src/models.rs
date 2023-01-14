use super::schema::*;
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Insertable, Serialize, Deserialize, Queryable, Debug)]
#[diesel(table_name = contracts)]
pub struct NewContract {
    pub uuid: String,
    pub state: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Queryable, Debug)]
pub struct Contract {
    pub id: i32,
    pub uuid: String,
    pub state: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug)]
#[diesel(table_name = contracts)]
pub struct UpdateContract {
    state: Option<String>,
    content: Option<String>,
}

#[derive(Insertable, Serialize, Deserialize, Queryable, Debug)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub event_id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Queryable, Debug)]
pub struct Event {
    pub id: i32,
    pub event_id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug)]
#[diesel(table_name = events)]
pub struct UpdateEvent {
    content: Option<String>,
}
