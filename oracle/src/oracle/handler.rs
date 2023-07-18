extern crate base64;
use crate::oracle::OracleError;
use dlc_clients::{
    EventRequestParams, EventsRequestParams, NewEvent, StorageApiClient, UpdateEvent,
};
use log::info;
use sled::{Config, Db};
use std::env;

extern crate futures;
extern crate tokio;

#[derive(Clone)]
pub struct EventHandler {
    key: String,
    pub sled_db: Option<Db>,
    pub storage_api: Option<StorageApiConn>,
}

impl EventHandler {
    pub fn new(key: String) -> Self {
        let sled;
        let storage_api_conn;
        let use_storage_api: bool = env::var("STORAGE_API_ENABLED")
            .unwrap_or("false".to_string())
            .parse()
            .unwrap();
        let storage_api_endpoint: String =
            env::var("STORAGE_API_ENDPOINT").unwrap_or("http://localhost:8100".to_string());
        info!("Storage api enabled: {}", use_storage_api);
        if use_storage_api {
            sled = None;
            let storage_api_client = StorageApiClient::new(storage_api_endpoint);
            storage_api_conn = Some(StorageApiConn::new(storage_api_client, key.clone()));
        } else {
            let oracle_events_db_path: String =
                env::var("ORACLE_EVENTS_DB_PATH").unwrap_or("".to_string());
            let path = match oracle_events_db_path.is_empty() {
                true => "events_db",
                false => &oracle_events_db_path,
            };
            info!("creating sled event database at {}", path);
            sled = Some(
                Config::new()
                    .path(path)
                    .cache_capacity(128 * 1024 * 1024)
                    .open()
                    .unwrap(),
            );
            storage_api_conn = None;
        }

        Self {
            key,
            sled_db: sled,
            storage_api: storage_api_conn,
        }
    }

    pub fn is_empty(&self) -> bool {
        if self.storage_api.is_some() {
            return false;
        } else {
            return self.sled_db.as_ref().unwrap().is_empty();
        }
    }
}

#[derive(Clone)]
pub struct StorageApiConn {
    pub client: StorageApiClient,
    key: String,
}

impl StorageApiConn {
    pub fn new(client: StorageApiClient, key: String) -> Self {
        Self { client, key }
    }

    pub async fn insert(
        &self,
        event_id: String,
        new_event: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, OracleError> {
        let new_content = base64::encode(new_event.clone());
        let event = self
            .client
            .get_event(EventRequestParams {
                key: self.key.clone(),
                event_id: event_id.clone(),
            })
            .await?;
        if event.is_some() {
            let update_event = UpdateEvent {
                content: new_content.clone(),
                event_id: event_id.clone(),
                key: self.key.clone(),
            };
            let res = self.client.update_event(update_event).await;
            match res {
                Ok(_) => Ok(Some(new_event.clone())),
                Err(err) => {
                    return Err(OracleError::StorageApiError(err));
                }
            }
        } else {
            let event = NewEvent {
                event_id: event_id.clone(),
                content: new_content.clone(),
                key: self.key.clone(),
            };
            let res = self.client.create_event(event).await;
            match res {
                Ok(_) => Ok(Some(new_event.clone())),
                Err(err) => {
                    return Err(OracleError::StorageApiError(err));
                }
            }
        }
    }

    // Use get_Event here
    pub async fn get(&self, event_id: String) -> Result<Option<Vec<u8>>, OracleError> {
        let event = self
            .client
            .get_event(EventRequestParams {
                event_id: event_id.clone(),
                key: self.key.clone(),
            })
            .await?;
        match event {
            Some(event) => {
                let res = base64::decode(event.content).unwrap();
                Ok(Some(res))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all(&self) -> Result<Option<Vec<(String, Vec<u8>)>>, OracleError> {
        let events = self
            .client
            .get_events(EventsRequestParams {
                key: self.key.clone(),
                event_id: None,
            })
            .await;
        let results = events
            .unwrap()
            .into_iter()
            .map(|event| {
                let content = base64::decode(event.content).unwrap();
                (event.event_id, content)
            })
            .collect();
        Ok(Some(results))
    }
}
