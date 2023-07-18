extern crate base64;
use crate::oracle::OracleError;
use dlc_clients::{
    EventRequestParams, EventsRequestParams, MemoryApiClient, NewEvent, StorageApiClient,
    UpdateEvent,
};
// use log::info;

extern crate futures;
// extern crate tokio;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! clog {
  ( $( $t:tt )* ) => {
      web_sys::console::log_1(&format!( $( $t )* ).into())
  }
}

#[derive(Clone)]
pub struct EventHandler {
    pub storage_api: Option<StorageApiConn>,
    pub memory_api: Option<MemoryApiConn>,
}

impl EventHandler {
    pub fn new(storage_api_enabled: bool, storage_api_endpoint: String, key: String) -> Self {
        clog!(
            "[EVENT_HANDLER] Storage api enabled: {}",
            storage_api_enabled
        );

        if storage_api_enabled && !storage_api_endpoint.is_empty() {
            let storage_api_client = StorageApiClient::new(storage_api_endpoint);
            let storage_api_conn = Some(StorageApiConn::new(storage_api_client, key));
            clog!("[EVENT_HANDLER] Storage api conn: {:?}", storage_api_conn);

            Self {
                storage_api: storage_api_conn,
                memory_api: None,
            }
        } else {
            Self {
                storage_api: None,
                memory_api: Some(MemoryApiConn::new(key)),
            }
        }
    }
}

#[derive(Clone)]
pub struct MemoryApiConn {
    pub client: MemoryApiClient,
    key: String,
}

impl MemoryApiConn {
    pub fn new(key: String) -> Self {
        let client = MemoryApiClient::new();
        Self { client, key }
    }

    pub async fn insert(
        &mut self,
        event_id: String,
        new_event: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, OracleError> {
        let new_content = base64::encode(new_event.clone());
        let event = self.client.get_event(event_id.clone()).await?;
        if event.is_some() {
            let update_event = UpdateEvent {
                content: new_content.clone(),
                event_id: event_id.clone(),
                key: self.key.clone(),
            };
            let res = self
                .client
                .update_event(event_id.clone(), update_event)
                .await;
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

    pub async fn get(&self, event_id: String) -> Result<Option<Vec<u8>>, OracleError> {
        let event = self.client.get_event(event_id.clone()).await?;
        if event.is_some() {
            let res = base64::decode(event.unwrap().content).unwrap();
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all(&self) -> Result<Option<Vec<(String, Vec<u8>)>>, OracleError> {
        let res_events = self.client.get_events().await.unwrap();
        let mut result: Vec<(String, Vec<u8>)> = vec![];
        for event in res_events {
            let content = base64::decode(event.content).unwrap();
            result.push((event.event_id, content));
        }
        return Ok(Some(result));
    }
}

#[derive(Debug, Clone)]
pub struct StorageApiConn {
    pub client: StorageApiClient,
    key: String,
}
impl StorageApiConn {
    pub fn new(client: StorageApiClient, key: String) -> Self {
        Self { client, key }
    }

    // Todo: Remove upsert functionality for simplicity
    pub async fn insert(
        &self,
        event_id: String,
        new_event: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, OracleError> {
        let new_content = base64::encode(new_event.clone());
        let event = match self
            .client
            .get_event(EventRequestParams {
                key: self.key.clone(),
                event_id: event_id.clone(),
            })
            .await
        {
            Ok(event) => event,
            Err(err) => {
                clog!("Error getting event: {:?}", err);
                return Err(OracleError::StorageApiError(err));
            }
        };

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

    pub async fn get(&self, event_id: String) -> Result<Option<Vec<u8>>, OracleError> {
        let event = self
            .client
            .get_event(EventRequestParams {
                key: self.key.clone(),
                event_id: event_id.clone(),
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
            .await
            .unwrap();
        let mut result: Vec<(String, Vec<u8>)> = vec![];
        for event in events {
            let content = base64::decode(event.content).unwrap();
            result.push((event.event_id, content));
        }
        return Ok(Some(result));
    }
}
