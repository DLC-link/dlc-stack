extern crate base64;
use crate::oracle::OracleError;
use dlc_clients::{
    EventRequestParams, EventsRequestParams, NewEvent, StorageApiClient, UpdateEvent,
};
use secp256k1_zkp::SecretKey;

extern crate futures;

macro_rules! clog {
  ( $( $t:tt )* ) => {
      web_sys::console::log_1(&format!( $( $t )* ).into())
  }
}

#[derive(Clone)]
pub struct EventHandler {
    pub storage_api: StorageApiConn,
}

impl EventHandler {
    pub fn new(storage_api_endpoint: String, public_key: String) -> Self {
        let storage_api_client = StorageApiClient::new(storage_api_endpoint);
        let storage_api_conn = StorageApiConn::new(storage_api_client, public_key);

        Self {
            storage_api: storage_api_conn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageApiConn {
    pub client: StorageApiClient,
    public_key: String,
}

impl StorageApiConn {
    pub fn new(client: StorageApiClient, public_key: String) -> Self {
        Self { client, public_key }
    }

    // Todo: Remove upsert functionality for simplicity
    pub async fn insert(
        &self,
        event_id: String,
        new_event: Vec<u8>,
        secret_key: SecretKey,
    ) -> Result<Option<Vec<u8>>, OracleError> {
        let new_content = base64::encode(new_event.clone());
        let event = match self
            .client
            .get_event(
                EventRequestParams {
                    key: self.public_key.clone(),
                    event_id: event_id.clone(),
                },
                secret_key,
            )
            .await
        {
            Ok(event) => event,
            Err(err) => {
                clog!("[WASM-ATTESTOR] Error getting event: {:?}", err);
                return Err(OracleError::StorageApiError(err));
            }
        };

        if event.is_some() {
            let update_event = UpdateEvent {
                content: new_content.clone(),
                event_id: event_id.clone(),
                key: self.public_key.clone(),
            };
            let res = self.client.update_event(update_event, secret_key).await;
            match res {
                Ok(_) => Ok(Some(new_event.clone())),
                Err(err) => Err(OracleError::StorageApiError(err)),
            }
        } else {
            let event = NewEvent {
                event_id: event_id.clone(),
                content: new_content.clone(),
                key: self.public_key.clone(),
            };
            let res = self.client.create_event(event, secret_key).await;
            match res {
                Ok(_) => Ok(Some(new_event.clone())),
                Err(err) => Err(OracleError::StorageApiError(err)),
            }
        }
    }

    pub async fn get(
        &self,
        event_id: String,
        secret_key: SecretKey,
    ) -> Result<Option<Vec<u8>>, OracleError> {
        let event = self
            .client
            .get_event(
                EventRequestParams {
                    key: self.public_key.clone(),
                    event_id: event_id.clone(),
                },
                secret_key,
            )
            .await?;

        match event {
            Some(event) => {
                let res = base64::decode(event.content).map_err(OracleError::Base64DecodeError)?;
                Ok(Some(res))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all(
        &self,
        secret_key: SecretKey,
    ) -> Result<Option<Vec<(String, Vec<u8>)>>, OracleError> {
        let events = self
            .client
            .get_events(
                EventsRequestParams {
                    key: self.public_key.clone(),
                    event_id: None,
                },
                secret_key,
            )
            .await
            .map_err(OracleError::StorageApiError)?;

        let mut result: Vec<(String, Vec<u8>)> = vec![];
        for event in events {
            let content = base64::decode(event.content).map_err(OracleError::Base64DecodeError)?;
            result.push((event.event_id, content));
        }
        Ok(Some(result))
    }
}
