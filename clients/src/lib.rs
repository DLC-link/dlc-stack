#![feature(async_fn_in_trait)]
#![deny(clippy::unwrap_used)]
extern crate serde;

use log::{debug, error};
use reqwest::{Client, Response};
use secp256k1_zkp::hashes::{sha256, Hash};
use secp256k1_zkp::{ecdsa, Message, Secp256k1, SecretKey};

use serde_json::{json, Value};
use std::fmt::{Debug, Formatter};
use std::time::Duration;
use std::{error, fmt};

pub mod async_storage_provider;
mod utils;

const REQWEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferRequest {
    pub uuid: String,
    pub accept_collateral: u64,
    pub offer_collateral: u64,
    pub total_outcomes: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SignedMessage {
    message: serde_json::Value,
    public_key: String,
    signature: String,
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

// implement from reqwest error trait for ApiError
impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError {
            message: e.to_string(),
            status: e
                .status()
                .unwrap_or(reqwest::StatusCode::BAD_REQUEST)
                .into(),
        }
    }
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
pub struct SignedContractsRequestParams {
    key: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::unwrap_or_skip"
    )]
    uuid: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::unwrap_or_skip"
    )]
    state: Option<String>,
    signature: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NewEvent {
    pub event_id: String,
    pub content: String,
    pub key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Event {
    pub id: usize,
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
struct SignedEventsRequestParams {
    key: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::unwrap_or_skip"
    )]
    event_id: Option<String>,
    signature: String,
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
        let mut client_builder = Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        {
            client_builder = client_builder.tcp_keepalive(Some(Duration::from_secs(20)));
            client_builder = client_builder.timeout(REQWEST_TIMEOUT);
        }
        Self {
            client: client_builder
                .build()
                .expect("Storage API Client should be able to create a reqwest client"),
            host,
        }
    }

    async fn build_signed_message(
        &self,
        secret_key: SecretKey,
        mut message: Value,
    ) -> Result<(String, SignedMessage), ApiError> {
        let nonce = self.request_nonce().await?;
        message["nonce"] = nonce.clone().into();

        let (sig, pub_key) = self.sign(secret_key, message.to_string());
        let message_body = SignedMessage {
            message,
            public_key: pub_key.to_string(),
            signature: sig.to_string(),
        };
        Ok((nonce, message_body))
    }

    fn sign(
        &self,
        secret_key: SecretKey,
        message: String,
    ) -> (ecdsa::Signature, secp256k1_zkp::PublicKey) {
        let signer = Secp256k1::new();
        let public_key = secret_key.public_key(&signer);
        let digest = Message::from(sha256::Hash::hash(message.as_bytes()));
        (signer.sign_ecdsa(&digest, &secret_key), public_key)
    }

    pub async fn request_nonce(&self) -> Result<String, ApiError> {
        let uri = format!("{}/request_nonce", String::as_str(&self.host.clone()));
        let res = self.client.get(uri).send().await?;
        let nonce = res.text().await?;
        Ok(nonce)
    }

    pub async fn get_contracts(
        &self,
        contract_req: ContractsRequestParams,
        secret_key: SecretKey,
    ) -> Result<Vec<Contract>, ApiError> {
        let uri = format!("{}/contracts", String::as_str(&self.host.clone()),);

        let nonce = self.request_nonce().await?;
        let (sig, _pubkey) = self.sign(secret_key, nonce.clone());
        let signed_request_params = SignedContractsRequestParams {
            key: contract_req.key.clone(),
            uuid: contract_req.uuid.clone(),
            state: contract_req.state.clone(),
            signature: sig.to_string(),
        };

        let res = self
            .client
            .get(uri)
            .header("authorization", nonce)
            .query(&json!(signed_request_params))
            .send()
            .await?;
        let status = res.status().into();
        let contracts = res.json::<Vec<Contract>>().await.map_err(|e| ApiError {
            message: format!(
                "get contracts failed, response from API not a list of contract objects, error: {}",
                e
            ),
            status,
        })?;
        Ok(contracts)
    }

    pub async fn get_contract(
        &self,
        contract_req: ContractRequestParams,
        secret_key: SecretKey,
    ) -> Result<Option<Contract>, ApiError> {
        debug!("getting contract with uuid: {}", contract_req.uuid);
        let contract = self
            .get_contracts(
                ContractsRequestParams {
                    uuid: Some(contract_req.uuid.clone()),
                    key: contract_req.key,
                    state: None,
                },
                secret_key,
            )
            .await?;
        Ok(contract.first().cloned())
    }

    pub async fn get_events(
        &self,
        event_req: EventsRequestParams,
        secret_key: SecretKey,
    ) -> Result<Vec<Event>, ApiError> {
        let uri = format!("{}/events", String::as_str(&self.host.clone()));
        debug!("getting events with request params: {:?}", event_req);

        let nonce = self.request_nonce().await?;
        let (sig, _pubkey) = self.sign(secret_key, nonce.clone());
        let signed_request_params = SignedEventsRequestParams {
            key: event_req.key.clone(),
            event_id: event_req.event_id.clone(),
            signature: sig.to_string(),
        };

        let res = self
            .client
            .get(uri)
            .header("authorization", nonce)
            .query(&signed_request_params)
            .send()
            .await?;
        let status = res.status().into();
        let events = res.json::<Vec<Event>>().await.map_err(|e| ApiError {
            message: format!(
                "get events failed, response from API not a list of event objects, error: {}",
                e
            ),
            status,
        })?;
        Ok(events)
    }

    pub async fn get_event(
        &self,
        event_req: EventRequestParams,
        secret_key: SecretKey,
    ) -> Result<Option<Event>, ApiError> {
        debug!("getting event with uuid: {}", event_req.event_id);
        let events = self
            .get_events(
                EventsRequestParams {
                    key: event_req.key.clone(),
                    event_id: Some(event_req.event_id.clone()),
                },
                secret_key,
            )
            .await?;
        Ok(events.first().cloned())
    }

    pub async fn create_contract(
        &self,
        contract: NewContract,
        secret_key: SecretKey,
    ) -> Result<Contract, ApiError> {
        let uri: String = format!("{}/contracts", String::as_str(&self.host.clone()));
        debug!("calling contract create on url: {:?}", uri);

        let (nonce, message_body) = self
            .build_signed_message(secret_key, json!(contract))
            .await?;

        let res = self
            .client
            .post(uri)
            .header("authorization", nonce)
            .json(&json!(message_body))
            .send()
            .await?;

        let status = res.status().into();
        let contract = res.json::<Contract>().await.map_err(|e| ApiError {
            message: format!(
                "Create contract failed, response from API not an contract object, error: {}",
                e
            ),
            status,
        })?;
        Ok(contract)
    }

    pub async fn create_event(
        &self,
        event: NewEvent,
        secret_key: SecretKey,
    ) -> Result<Event, ApiError> {
        let uri = format!("{}/events", String::as_str(&self.host.clone()));
        debug!("calling event create on url: {:?}", uri);

        let (nonce, message_body) = self.build_signed_message(secret_key, json!(event)).await?;

        let res = self
            .client
            .post(uri)
            .header("authorization", nonce)
            .json(&message_body)
            .send()
            .await?;
        let status = res.status().into();
        let event = res.json::<Event>().await.map_err(|e| ApiError {
            message: format!(
                "Create event failed, response from API not an event object, error: {}",
                e
            ),
            status,
        })?;
        Ok(event)
    }

    pub async fn update_event(
        &self,
        event: UpdateEvent,
        secret_key: SecretKey,
    ) -> Result<(), ApiError> {
        let uri = format!("{}/events", String::as_str(&self.host.clone()));
        debug!("calling event update on url: {:?}", uri);

        let (nonce, message_body) = self.build_signed_message(secret_key, json!(event)).await?;

        let res = self
            .client
            .put(uri)
            .header("authorization", nonce)
            .json(&message_body)
            .send()
            .await?;
        let status = res.status().into();
        match res
            .json::<EffectedNumResponse>()
            .await
            .map_err(|e| ApiError {
                message: format!(
                    "Updating event failed, response from API not a number, error: {}",
                    e
                ),
                status,
            })?
            .effected_num
        {
            0 => Err(ApiError {
                message: "No event updated".to_string(),
                status,
            }),
            1 => Ok(()),
            _ => {
                error!("More than one event updated");
                Ok(())
            }
        }
    }

    pub async fn update_contract(
        &self,
        contract: UpdateContract,
        secret_key: SecretKey,
    ) -> Result<(), ApiError> {
        let uri = format!("{}/contracts", String::as_str(&self.host.clone()));
        debug!("calling contract update on url: {:?}", uri);
        let (nonce, message_body) = self
            .build_signed_message(secret_key, json!(contract))
            .await?;
        let res = self
            .client
            .put(uri)
            .header("authorization", nonce)
            .json(&json!(message_body))
            .send()
            .await?;
        let status = res.status().into();
        match res
            .json::<EffectedNumResponse>()
            .await
            .map_err(|e| ApiError {
                message: format!(
                    "Updating contract failed, response from API not a number, error: {}",
                    e
                ),
                status,
            })?
            .effected_num
        {
            0 => Err(ApiError {
                message: "No contract updated".to_string(),
                status,
            }),
            1 => Ok(()),
            _ => {
                error!("More than one contract updated");
                Ok(())
            }
        }
    }

    pub async fn delete_event(
        &self,
        event: EventRequestParams,
        secret_key: SecretKey,
    ) -> Result<(), ApiError> {
        let uri = format!("{}/event", String::as_str(&self.host.clone()));
        debug!("calling event delete on url: {:?}", uri);

        let (nonce, message_body) = self.build_signed_message(secret_key, json!(event)).await?;

        let res = self
            .client
            .delete(uri)
            .header("authorization", nonce)
            .json(&message_body)
            .send()
            .await?;
        let status = res.status().into();
        match res
            .json::<EffectedNumResponse>()
            .await
            .map_err(|e| ApiError {
                message: format!(
                    "Deleting event failed, response from API not a number, error: {}",
                    e
                ),
                status,
            })?
            .effected_num
        {
            0 => Err(ApiError {
                message: "No event deleted".to_string(),
                status,
            }),
            1 => Ok(()),
            _ => {
                error!("More than one event deleted");
                Ok(())
            }
        }
    }

    pub async fn delete_contract(
        &self,
        contract: ContractRequestParams,
        secret_key: SecretKey,
    ) -> Result<(), ApiError> {
        let uri = format!("{}/contract", String::as_str(&self.host.clone()));
        debug!("calling contract delete on url: {:?}", uri);
        let (nonce, message_body) = self
            .build_signed_message(secret_key, json!(contract))
            .await?;
        let res = self
            .client
            .delete(uri)
            .header("authorization", nonce)
            .json(&json!(message_body))
            .send()
            .await?;
        let status = res.status().into();
        match res
            .json::<EffectedNumResponse>()
            .await
            .map_err(|e| ApiError {
                message: format!(
                    "Deleting contract failed, response from API not a number, error: {}",
                    e
                ),
                status,
            })?
            .effected_num
        {
            0 => Err(ApiError {
                message: "No contract deleted".to_string(),
                status,
            }),
            1 => Ok(()),
            _ => {
                error!("More than one contract deleted");
                Ok(())
            }
        }
    }

    // For testing only, should be removed
    // pub async fn delete_contracts(&self, key: String) -> Result<(), ApiError> {
    //     self.delete_resources("contracts".to_string(), key).await
    // }

    // pub async fn delete_events(&self, key: String) -> Result<(), ApiError> {
    //     self.delete_resources("events".to_string(), key).await
    // }

    // For testing only, should be removed
    // async fn delete_resources(&self, path: String, key: String) -> Result<(), ApiError> {
    //     let uri = format!(
    //         "{}/{}/{}",
    //         String::as_str(&self.host.clone()),
    //         path.as_str(),
    //         key.clone()
    //     );

    //     let res = self.client.delete(uri).send().await?;
    //     let status = res.status();
    //     match status.clone() {
    //         StatusCode::OK => Ok(()),
    //         _ => {
    //             let msg: String = res.text().await.map_err(|e| ApiError {
    //                 message: e.to_string(),
    //                 status: status.clone().as_u16(),
    //             })?;
    //             Err(ApiError {
    //                 message: msg,
    //                 status: status.clone().as_u16(),
    //             })
    //         }
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    use crate::StorageApiClient;
    use bdk::keys::bip39::{Language, Mnemonic, WordCount};
    use bdk::keys::{DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey};
    use bdk::miniscript::Segwitv0;
    use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPubKey};
    use secp256k1_zkp::ecdsa::Signature;
    use serde_json::json;

    #[actix_rt::test]
    async fn test_build_signed_message() {
        // Setup API Client
        let mut server = mockito::Server::new();
        let server_url = server.url();

        server
            .mock("GET", "/request_nonce")
            .with_status(200)
            .with_body("abcde")
            .create_async()
            .await;

        let client = StorageApiClient::new(server_url);

        // Create ExtendedPrivateKey
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let mnemonic: GeneratedKey<_, Segwitv0> =
            Mnemonic::generate((WordCount::Words24, Language::English))
                .expect("should be able to generate mnemonic");
        let mnemonic = mnemonic.into_key();

        let xkey: ExtendedKey = (mnemonic.clone(), None)
            .into_extended_key()
            .expect("should be able to generate extended key");
        let xpriv = xkey
            .into_xprv(bitcoin::Network::Testnet)
            .expect("should be able to generate extended private key");

        let external_derivation_path = DerivationPath::from_str("m/44h/0h/0h/0")
            .expect("should be able to parse derivation path");

        let derived_xpriv = xpriv
            .derive_priv(
                &secp,
                &external_derivation_path.extend([
                    ChildNumber::Normal { index: 0 },
                    ChildNumber::Normal { index: 0 },
                ]),
            )
            .expect("should be able to derive private key");

        let secret_key = derived_xpriv.private_key;
        let public_key = ExtendedPubKey::from_priv(&secp, &derived_xpriv);

        // Create contract without nonce
        let expected_nonce: String = "abcde".to_string();

        let contract_wo_nonce = json!({
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        // Create contract with nonce
        let mut contract_w_nonce = contract_wo_nonce.clone();
        contract_w_nonce["nonce"] = expected_nonce.clone().into();

        // Build signed message
        let (nonce, signed_message) = client
            .build_signed_message(secret_key, contract_wo_nonce)
            .await
            .expect("should be able to build signed message");

        // Assert signed message
        assert_eq!(signed_message.message, contract_w_nonce);

        // Verify nonce and signature
        assert_eq!(nonce, expected_nonce);
        let digest = Message::from(sha256::Hash::hash(contract_w_nonce.to_string().as_bytes()));

        assert!(secp
            .verify_ecdsa(
                &digest,
                &Signature::from_str(&signed_message.signature)
                    .expect("can make signature from string"),
                &public_key.public_key
            )
            .is_ok());
    }
}
