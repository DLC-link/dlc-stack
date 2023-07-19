extern crate serde;

use log::{debug, info, warn};
use reqwest::{Client, Error, Response, StatusCode, Url};
use std::fmt::{Debug, Formatter};
use std::{error, fmt};

use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferRequest {
    pub uuid: String,
    pub accept_collateral: u64,
    pub offer_collateral: u64,
    pub total_outcomes: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptMessage {
    pub accept_message: String,
}

#[derive(Debug)]
pub struct ApiResult {
    pub status: u16,
    pub response: Response,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub message: String,
    pub status: u16,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiError: {} - {}", self.status, self.message)
    }
}

impl error::Error for ApiError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Contract {
    pub id: i32,
    pub uuid: String,
    pub state: String,
    pub content: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NewContract {
    pub uuid: String,
    pub state: String,
    pub content: String,
    pub key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UpdateContract {
    pub uuid: String,
    pub state: Option<String>,
    pub content: Option<String>,
    pub key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ContractRequestParams {
    pub key: String,
    pub uuid: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ContractsRequestParams {
    pub key: String,
    pub uuid: Option<String>,
    pub state: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NewEvent {
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Event {
    pub id: i32,
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UpdateEvent {
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct EventRequestParams {
    pub key: String,
    pub event_id: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct EventsRequestParams {
    pub key: String,
    pub event_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct EffectedNumResponse {
    pub effected_num: u32,
}

pub struct WalletBackendClient {
    client: Client,
    host: String,
}

impl Default for WalletBackendClient {
    fn default() -> Self {
        Self::new("http://localhost:8085".to_string())
    }
}

impl Debug for WalletBackendClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.host)
    }
}

impl WalletBackendClient {
    pub fn new(host: String) -> Self {
        Self {
            client: Client::new(),
            host: host,
        }
    }

    pub async fn post_offer_and_accept(
        &self,
        offer_request: OfferRequest,
    ) -> Result<ApiResult, Error> {
        let uri = format!("{}/offer", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();
        let res = self.client.post(url).json(&offer_request).send().await?;
        let result = ApiResult {
            status: res.status().as_u16(),
            response: res,
        };
        Ok(result)
    }

    pub async fn put_accept(&self, accept_request: AcceptMessage) -> Result<ApiResult, Error> {
        let uri = format!("{}/offer/accept", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();
        let res = self.client.put(url).json(&accept_request).send().await?;
        let result = ApiResult {
            status: res.status().as_u16(),
            response: res,
        };
        Ok(result)
    }
}

#[derive(Clone)]
pub struct MemoryApiClient {
    events: HashMap<String, String>,
}

impl MemoryApiClient {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    pub async fn get_events(&self) -> Result<Vec<Event>, ApiError> {
        let mut events: Vec<Event> = Vec::new();
        for (uuid, content) in self.events.iter() {
            events.push(Event {
                id: 1,
                event_id: uuid.to_string(),
                content: content.to_string(),
                key: "mykey".to_string(),
            });
        }
        Ok(events)
    }

    pub async fn get_event(&self, uuid: String) -> Result<Option<Event>, ApiError> {
        let res = self.events.get(&uuid);
        if res.is_none() {
            return Ok(None);
        }
        Ok(Some(Event {
            id: 1,
            event_id: uuid,
            content: res.unwrap().to_string(),
            key: "mykey".to_string(),
        }))
    }

    pub async fn create_event(&mut self, event: NewEvent) -> Result<Event, ApiError> {
        self.events
            .insert(event.event_id.clone(), event.content.clone());
        Ok(Event {
            id: 1,
            event_id: event.event_id,
            content: event.content,
            key: "mykey".to_string(),
        })
    }

    pub async fn update_event(&mut self, uuid: String, event: UpdateEvent) -> Result<(), ApiError> {
        let res = self.events.get(&uuid);
        if res.is_none() {
            return Err(ApiError {
                message: "Event not found".to_string(),
                status: 404,
            });
        }
        self.events.remove(&uuid);
        self.events.insert(uuid, event.content);
        return Ok(());
    }

    pub async fn delete_event(&self, _uuid: String) -> Result<(), ApiError> {
        unimplemented!()
    }

    pub async fn delete_events(&self) -> Result<(), ApiError> {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct StorageApiClient {
    client: Client,
    host: String,
}

impl Default for StorageApiClient {
    fn default() -> Self {
        Self::new("http://localhost:8100".to_string())
    }
}

impl Debug for StorageApiClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.host)
    }
}

impl StorageApiClient {
    pub fn new(host: String) -> Self {
        Self {
            client: Client::new(),
            host: host,
        }
    }

    pub async fn get_contracts(
        &self,
        contract: ContractsRequestParams,
    ) -> Result<Vec<Contract>, ApiError> {
        let uri = format!("{}/contracts", String::as_str(&self.host.clone()),);
        debug!("get_contracts calling for URI: {}", uri);
        let url = Url::parse(uri.as_str()).unwrap();

        let res = match self.client.get(url).query(&contract).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };

        let status = res.status();
        match status.clone() {
            StatusCode::OK => {
                let contracts: Vec<Contract> = res.json().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Ok(contracts)
            }
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn get_contract(
        &self,
        contract: ContractRequestParams,
    ) -> Result<Option<Contract>, ApiError> {
        info!("getting contract with uuid: {}", contract.uuid);

        let contracts = self
            .get_contracts(ContractsRequestParams {
                uuid: Some(contract.uuid.clone()),
                key: contract.key,
                state: None,
            })
            .await?;

        match contracts.len() {
            0 => {
                info!("Contract not found with id: {}", contract.uuid);
                Ok(None)
            }
            1 => {
                info!("Contract found with id: {}", contract.uuid);
                Ok(Some(contracts.first().unwrap().clone()))
            }
            _ => {
                warn!("More than one contract found with id: {}", contract.uuid);
                Err(ApiError {
                    message: "Duplicate contracts found".to_string(),
                    status: 400,
                })
            }
        }
    }

    pub async fn get_events(&self, event: EventsRequestParams) -> Result<Vec<Event>, ApiError> {
        let uri = format!("{}/events", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();
        let res = match self.client.get(url).query(&event).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };

        let status = res.status();
        match status.clone() {
            StatusCode::OK => {
                let events: Vec<Event> = res.json().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Ok(events)
            }
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn get_event(&self, event: EventRequestParams) -> Result<Option<Event>, ApiError> {
        info!("getting contract with uuid: {}", event.event_id);

        let events = self
            .get_events(EventsRequestParams {
                key: event.key.clone(),
                event_id: Some(event.event_id.clone()),
            })
            .await?;

        match events.len() {
            0 => {
                info!("Event not found with id: {}", event.event_id);
                Ok(None)
            }
            1 => {
                info!("Event found with id: {}", event.event_id);
                Ok(Some(events.first().unwrap().clone()))
            }
            _ => {
                warn!("More than one contract found with id: {}", event.event_id);
                Err(ApiError {
                    message: "Duplicate events found".to_string(),
                    status: 400,
                })
            }
        }
    }

    pub async fn create_contract(&self, contract: NewContract) -> Result<Contract, ApiError> {
        let uri = format!("{}/contracts", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();
        let res = match self.client.post(url).json(&contract).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => {
                let contract: Contract = res.json().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Ok(contract)
            }
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn create_event(&self, event: NewEvent) -> Result<Event, ApiError> {
        let uri = format!("{}/events", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();
        let res = match self.client.post(url).json(&event).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => {
                let event: Event = res.json().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Ok(event)
            }
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn update_event(&self, event: UpdateEvent) -> Result<(), ApiError> {
        let uri = format!("{}/events", String::as_str(&self.host.clone()),);
        let url = Url::parse(uri.as_str()).unwrap();
        let res = match self.client.put(url).json(&event).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => match res
                .json::<EffectedNumResponse>()
                .await
                .map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?
                .effected_num
            {
                0 => Err(ApiError {
                    message: "No event updated".to_string(),
                    status: status.clone().as_u16(),
                }),
                1 => Ok(()),
                _ => {
                    warn!("More than one event updated");
                    Ok(())
                }
            },
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn update_contract(&self, contract: UpdateContract) -> Result<(), ApiError> {
        let uri = format!("{}/contracts", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();

        info!("calling url: {:?}", url);

        let res = match self.client.put(url).json(&contract).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => match res
                .json::<EffectedNumResponse>()
                .await
                .map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?
                .effected_num
            {
                0 => Err(ApiError {
                    message: "No contract updated".to_string(),
                    status: status.clone().as_u16(),
                }),
                1 => Ok(()),
                _ => {
                    warn!("More than one contract updated");
                    Ok(())
                }
            },
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    // key for all these too
    pub async fn delete_event(&self, event: EventRequestParams) -> Result<(), ApiError> {
        let uri = format!("{}/event", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();

        info!("calling delete on url: {:?}", url);

        let res = match self.client.delete(url).json(&event).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => match res
                .json::<EffectedNumResponse>()
                .await
                .map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?
                .effected_num
            {
                0 => Err(ApiError {
                    message: "No event deleted".to_string(),
                    status: status.clone().as_u16(),
                }),
                1 => Ok(()),
                _ => {
                    warn!("More than one event deleted");
                    Ok(())
                }
            },
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn delete_contract(&self, contract: ContractRequestParams) -> Result<(), ApiError> {
        let uri = format!("{}/contract", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();

        info!("calling delete on url: {:?}", url);

        let res = match self.client.delete(url).json(&contract).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => match res
                .json::<EffectedNumResponse>()
                .await
                .map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?
                .effected_num
            {
                0 => Err(ApiError {
                    message: "No contract deleted".to_string(),
                    status: status.clone().as_u16(),
                }),
                1 => Ok(()),
                _ => {
                    warn!("More than one contract deleted");
                    Ok(())
                }
            },
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }

    pub async fn delete_contracts(&self, key: String) -> Result<(), ApiError> {
        self.delete_resources("contracts".to_string(), key).await
    }

    pub async fn delete_events(&self, key: String) -> Result<(), ApiError> {
        self.delete_resources("events".to_string(), key).await
    }

    // TODO: for testing only, should be removed
    async fn delete_resources(&self, path: String, key: String) -> Result<(), ApiError> {
        let uri = format!(
            "{}/{}/{}",
            String::as_str(&self.host.clone()),
            path.as_str(),
            key.clone()
        );
        let url = Url::parse(uri.as_str()).unwrap();
        let res = match self.client.delete(url).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        match status.clone() {
            StatusCode::OK => Ok(()),
            _ => {
                let msg: String = res.text().await.map_err(|e| ApiError {
                    message: e.to_string(),
                    status: status.clone().as_u16(),
                })?;
                Err(ApiError {
                    message: msg,
                    status: status.clone().as_u16(),
                })
            }
        }
    }
}
