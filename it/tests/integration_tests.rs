use cucumber::{given, then, when, Parameter, World};
use derive_more::{Deref, FromStr};
use dlc_clients::{AcceptMessage, ApiError, ApiResult, OfferRequest, WalletBackendClient};
use std::collections::HashMap;
use tokio::runtime::Runtime;

use reqwest::{Client, Error, Response, StatusCode, Url};
use std::fmt::{Debug, Formatter};

pub struct OracleBackendClient {
    client: Client,
    host: String,
}

impl Default for OracleBackendClient {
    fn default() -> Self {
        Self::new("http://localhost:8080".to_string())
    }
}

impl Debug for OracleBackendClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.host)
    }
}

impl OracleBackendClient {
    pub fn new(host: String) -> Self {
        Self {
            client: Client::new(),
            host: host,
        }
    }

    pub async fn create_event(&self, uuid: String) -> Result<ApiResult, Error> {
        let uri = format!(
            "{}/v1/create_event/{}?maturation=2022-10-08T13:48:00Z",
            String::as_str(&self.host.clone()),
            uuid.as_str()
        );
        let url = Url::parse(uri.as_str()).unwrap();
        let res = self.client.get(url).send().await?;
        let result = ApiResult {
            status: res.status().as_u16(),
            response: res,
        };
        Ok(result)
    }

    pub async fn get_attestation(&self, uuid: String, outcome: String) -> Result<ApiResult, Error> {
        let uri = format!(
            "{}/v1/attest/{}?outcome={}",
            String::as_str(&self.host.clone()),
            uuid.as_str(),
            outcome.as_str()
        );
        let url = Url::parse(uri.as_str()).unwrap();
        let res = self.client.get(url).send().await?;
        let result = ApiResult {
            status: res.status().as_u16(),
            response: res,
        };
        Ok(result)
    }

    pub async fn get_public_key(&self) -> Result<String, ApiError> {
        let uri = format!("{}/v1/publickey", String::as_str(&self.host.clone()));
        let url = Url::parse(uri.as_str()).unwrap();
        let res = match self.client.get(url).send().await {
            Ok(result) => result,
            Err(e) => {
                return Err(ApiError {
                    message: e.to_string(),
                    status: 0,
                })
            }
        };
        let status = res.status();
        if status.is_success() {
            let status_clone = status.clone();
            let key_resp: String = res.text().await.map_err(|e| ApiError {
                message: e.to_string(),
                status: status_clone.as_u16(),
            })?;
            Ok(key_resp)
        } else {
            let status_clone = status.clone();
            let msg: String = res.text().await.map_err(|e| ApiError {
                message: e.to_string(),
                status: status_clone.as_u16(),
            })?;
            Err(ApiError {
                message: msg,
                status: status_clone.as_u16(),
            })
        }
    }
}
#[derive(Deref, FromStr, Parameter)]
#[param(regex = r"\d+", name = "u64")]
struct CustomU64(u64);

#[derive(Debug, Default, World)]
pub struct DlcLinkWorld {
    wallet_client: WalletBackendClient,
    oracle_client: OracleBackendClient,
    collected_responses: HashMap<String, ApiResult>,
}

#[given(expr = "a wallet backend client with address {word}")]
fn create_wallet_client(world: &mut DlcLinkWorld, address: String) {
    world.wallet_client = WalletBackendClient::new(address);
}

#[given(expr = "an oracle backend client with address {word}")]
fn create_oracle_client(world: &mut DlcLinkWorld, address: String) {
    world.oracle_client = OracleBackendClient::new(address);
}

#[when(expr = "accept message: {word} as '{word}'")]
async fn wallet_accept_message(world: &mut DlcLinkWorld, accept_message: String, context: String) {
    let accept_msg_request = AcceptMessage {
        accept_message: accept_message.to_string(),
    };
    let res = world.wallet_client.put_accept(accept_msg_request);
    world
        .collected_responses
        .insert(context.clone(), res.await.unwrap());
}

#[when(
    expr = "creating an offer request '{word}' with uuid {word}, accept_collateral: {u64} and offer_collateral: {u64}"
)]
async fn create_offer(
    world: &mut DlcLinkWorld,
    context: String,
    uuid: String,
    accept_collateral: CustomU64,
    offer_collateral: CustomU64,
) {
    let offer_request = OfferRequest {
        uuid: uuid.to_string(),
        accept_collateral: *accept_collateral,
        offer_collateral: *offer_collateral,
        total_outcomes: 1,
    };
    let res = world.wallet_client.post_offer_and_accept(offer_request);
    world
        .collected_responses
        .insert(context.clone(), res.await.unwrap());
}

#[when(expr = "creating a new oracle event '{word}' with uuid {word}")]
async fn create_event(world: &mut DlcLinkWorld, context: String, uuid: String) {
    let res = world.oracle_client.create_event(uuid.to_string());
    world
        .collected_responses
        .insert(context.clone(), res.await.unwrap());
}

#[when(expr = "getting an attestation '{word}' with uuid {word} and outcome: {word}")]
async fn get_attest(world: &mut DlcLinkWorld, context: String, uuid: String, outcome: String) {
    let res = world
        .oracle_client
        .get_attestation(uuid.to_string(), outcome.to_string());
    world
        .collected_responses
        .insert(context.clone(), res.await.unwrap());
}

#[then(expr = "expected status code for '{word}' is {u64}")]
fn expected_offer_result(world: &mut DlcLinkWorld, context: String, status_code: CustomU64) {
    let api_res = world.collected_responses.get(&context).unwrap();
    assert_eq!(*status_code, api_res.status as u64);
}

fn main() {
    Runtime::new()
        .unwrap()
        .block_on(DlcLinkWorld::run("tests/features"));
}
