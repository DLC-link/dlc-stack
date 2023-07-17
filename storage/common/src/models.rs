use super::schema::*;
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Insertable, Serialize, Deserialize, Queryable, Debug)]
#[diesel(table_name = contracts)]
pub struct NewContract {
    pub uuid: String,
    pub state: String,
    pub content: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Queryable, Debug)]
pub struct Contract {
    pub id: i32,
    pub uuid: String,
    pub state: String,
    pub content: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug, Clone)]
#[diesel(table_name = contracts)]
pub struct UpdateContract {
    pub uuid: String,
    pub state: Option<String>,
    pub content: Option<String>,
    pub key: String,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug)]
#[diesel(table_name = contracts)]
pub struct DeleteContract {
    pub uuid: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct ContractRequestParams {
    pub key: String,
    pub uuid: Option<String>,
    pub state: Option<String>,
}

#[derive(Insertable, Serialize, Deserialize, Queryable, Debug)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Queryable, Debug)]
pub struct Event {
    pub id: i32,
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug, Clone)]
#[diesel(table_name = events)]
pub struct UpdateEvent {
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, AsChangeset, Debug, Clone)]
#[diesel(table_name = events)]
pub struct DeleteEvent {
    pub event_id: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct EventRequestParams {
    pub key: String,
    pub event_id: Option<String>,
}
