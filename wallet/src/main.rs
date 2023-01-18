#![allow(unreachable_code)]
extern crate log;

#[macro_use]
extern crate rouille;

use std::{
    collections::HashMap,
    env, panic,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    vec,
};

use bitcoin_rpc_provider::BitcoinCoreProvider;
use bitcoincore_rpc::{Auth, Client};
use dlc_manager::{
    contract::{
        contract_input::{ContractInput, ContractInputInfo, OracleInput},
        Contract,
    },
    manager::Manager,
    Oracle, Storage, SystemTimeProvider, Wallet,
};
use dlc_messages::{AcceptDlc, Message};
use log::{debug, info, warn};

use crate::storage::storage_provider::StorageProvider;
use oracle_client::P2PDOracleClient;
use rouille::Response;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utils::get_numerical_contract_info;

mod oracle_client;
mod storage;
mod utils;
#[macro_use]
mod macros;

type DlcManager<'a> = Manager<
    Arc<BitcoinCoreProvider>,
    Arc<BitcoinCoreProvider>,
    Box<StorageProvider>,
    Arc<P2PDOracleClient>,
    Arc<SystemTimeProvider>,
>;

const NUM_CONFIRMATIONS: u32 = 2;
const COUNTER_PARTY_PK: &str = "02fc8e97419286cf05e5d133f41ff6d51f691dda039e9dc007245a421e2c7ec61c";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    message: String,
    code: Option<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorsResponse {
    errors: Vec<ErrorResponse>,
    status: u64,
}

fn main() {
    env_logger::init();
    // let auth = Auth::UserPass(
    //     "testuser".to_string(),
    //     "lq6zequb-gYTdF2_ZEUtr8ywTXzLYtknzWU4nV8uVoo=".to_string(),
    // );
    // let url = "http://localhost:18443/wallet/alice"; - localhost
    // let url = "http://54.147.153.106:18443/"; - devnet

    let oracle_url: String = env::var("ORACLE_URL").unwrap_or("http://localhost:8080".to_string());
    let rpc_user: String = env::var("RPC_USER").unwrap_or("testuser".to_string());
    let rpc_pass: String =
        env::var("RPC_PASS").unwrap_or("lq6zequb-gYTdF2_ZEUtr8ywTXzLYtknzWU4nV8uVoo=".to_string());
    let btc_rpc_url: String =
        env::var("BTC_RPC_URL").unwrap_or("localhost:18443/wallet/alice".to_string());
    let funded_url: String = env::var("FUNDED_URL")
        .unwrap_or("https://stacks-observer-mocknet.herokuapp.com/funded".to_string());

    let mut funded_uuids: Vec<String> = vec![];

    let auth = Auth::UserPass(rpc_user, rpc_pass);
    let rpc = Client::new(&format!("http://{}", btc_rpc_url), auth.clone()).unwrap();
    let bitcoin_core = Arc::new(BitcoinCoreProvider { client: rpc });
    let p2p_client: P2PDOracleClient = retry!(
        P2PDOracleClient::new(&oracle_url),
        10,
        "oracle client creation"
    );
    let oracle = Arc::new(p2p_client);
    let oracles: HashMap<secp256k1_zkp::schnorrsig::PublicKey, Arc<P2PDOracleClient>> =
        HashMap::from([(oracle.get_public_key(), oracle.clone())]);
    let store = StorageProvider::new();
    let time_provider = SystemTimeProvider {};
    let manager = Arc::new(Mutex::new(Manager::new(
        Arc::clone(&bitcoin_core),
        Arc::clone(&bitcoin_core),
        Box::new(store),
        oracles,
        Arc::new(time_provider),
    )));

    let man2 = manager.clone();
    info!("periodic_check loop thread starting");
    thread::spawn(move || loop {
        check_close(
            man2.clone(),
            bitcoin_core.clone(),
            funded_url.clone(),
            &mut funded_uuids,
        );
        thread::sleep(Duration::from_millis(5000));
    });

    rouille::start_server("0.0.0.0:8085", move |request| {
        router!(request,
                (GET) (/cleanup) => {
                    let contract_cleanup_enabled: bool = env::var("CONTRACT_CLEANUP_ENABLED")
                        .unwrap_or("false".to_string())
                        .parse().unwrap();
                    if contract_cleanup_enabled {
                        info!("Call cleanup contract offers.");
                        delete_all_offers(manager.clone(), Response::empty_204())
                    } else {
                        info!("Call cleanup contract offers feature disabled.");
                        Response::empty_400()
                    }
                },
                (POST) (/offer) => {
                    info!("Call POST (create) offer {:?}", request);
                    #[derive(Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct OfferRequest {
                        uuid: String,
                        accept_collateral: u64,
                        offer_collateral: u64,
                        total_outcomes: u64
                    }
                    let req: OfferRequest = try_or_400!(rouille::input::json_input(request));
                    add_access_control_headers(create_new_offer(manager.clone(), oracle.clone(), req.uuid, req.accept_collateral, req.offer_collateral, req.total_outcomes))
                },
                (OPTIONS) (/offer) => {
                    add_access_control_headers(Response::empty_204())
                },
                (OPTIONS) (/offer/accept) => {
                    add_access_control_headers(Response::empty_204())
                },
                (PUT) (/offer/accept) => {
                    info!("Call PUT (accept) offer {:?}", request);
                    #[derive(Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct AcceptOfferRequest {
                        accept_message: String,
                    }
                    let json: AcceptOfferRequest = try_or_400!(rouille::input::json_input(request));
                    info!("Accept message: {}", json.accept_message.clone());
                    let accept_dlc: AcceptDlc = match serde_json::from_str(&json.accept_message)
                    {
                        Ok(dlc) => dlc,
                        Err(e) => return add_access_control_headers(Response::json(&ErrorsResponse{status: 400, errors: vec![ErrorResponse{message: e.to_string(), code: None}]}).with_status_code(400)),
                    };
                    accept_offer(accept_dlc, manager.clone())
                },
                _ => rouille::Response::empty_404()
        )
    });
}

enum OfferType {
    Enumerated,
    Numerical,
}

enum Error {
    BadError(String),
}

impl FromStr for OfferType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enumerated" => Ok(OfferType::Enumerated),
            "numerical" => Ok(OfferType::Numerical),
            _ => Err(Error::BadError("Unknown contract type".to_string())),
        }
    }
}

fn check_close(
    manager: Arc<Mutex<DlcManager>>,
    wallet: Arc<BitcoinCoreProvider>,
    funded_url: String,
    funded_uuids: &mut Vec<String>,
) -> Response {
    let mut collected_response = json!({});
    let mut man = manager.lock().unwrap();

    man.periodic_check().unwrap();

    let store = man.get_store();

    collected_response["signed_contracts"] = store
        .get_signed_contracts()
        .unwrap()
        .iter()
        .map(|c| {
            let confirmations = match wallet
                .get_transaction_confirmations(&c.accepted_contract.dlc_transactions.fund.txid())
            {
                Ok(confirms) => confirms,
                Err(_) => 0,
            };
            if confirmations >= NUM_CONFIRMATIONS {
                let uuid = c.accepted_contract.offered_contract.contract_info[0]
                    .oracle_announcements[0]
                    .oracle_event
                    .event_id
                    .clone();
                if !funded_uuids.contains(&uuid) {
                    let mut post_body = HashMap::new();
                    post_body.insert("uuid", &uuid);

                    let client = match reqwest::blocking::Client::builder()
                        .use_rustls_tls()
                        .build()
                    {
                        Ok(c) => c,
                        Err(_) => panic!("Unable to create a new reqwest client"),
                    };
                    let res = client.post(&funded_url).json(&post_body).send();

                    match res {
                        Ok(res) => {
                            funded_uuids.push(uuid.clone());
                            info!("Success setting funded to true: {}, {}", uuid, res.status());
                        }
                        Err(e) => {
                            info!("Error setting funded to true: {}: {}", uuid, e.to_string());
                        }
                    }
                }
            }
            c.accepted_contract.get_contract_id_string()
        })
        .collect();

    collected_response["confirmed_contracts"] = store
        .get_confirmed_contracts()
        .unwrap()
        .iter()
        .map(|c| c.accepted_contract.get_contract_id_string())
        .collect();

    collected_response["preclosed_contracts"] = store
        .get_preclosed_contracts()
        .unwrap()
        .iter()
        .map(|c| c.signed_contract.accepted_contract.get_contract_id_string())
        .collect();

    let mut closed_contracts: Vec<String> = Vec::new();
    for val in store.get_contracts().unwrap().iter() {
        if let Contract::Closed(c) = val {
            closed_contracts.push(c.signed_contract.accepted_contract.get_contract_id_string());
        }
    }
    collected_response["closed_contracts"] = closed_contracts.into();

    Response::json(&collected_response)
}

fn create_new_offer(
    manager: Arc<Mutex<DlcManager>>,
    oracle: Arc<P2PDOracleClient>,
    event_id: String,
    accept_collateral: u64,
    offer_collateral: u64,
    total_outcomes: u64,
) -> Response {
    let (_event_descriptor, descriptor) =
        get_numerical_contract_info(accept_collateral, offer_collateral, total_outcomes);
    let announcement_res = oracle.get_announcement(&event_id);
    info!(
        "Creating new offer with event id: {}, accept collateral: {}, offer_collateral: {}",
        event_id.clone(),
        accept_collateral,
        offer_collateral
    );
    let maturity = match announcement_res {
        Ok(a) => a.oracle_event.event_maturity_epoch,
        Err(_e) => {
            return Response::json(&ErrorsResponse {
                status: 400,
                errors: vec![ErrorResponse {
                    message: "OracleEventNotFoundError".to_string(),
                    code: None,
                }],
            })
            .with_status_code(400)
        }
    };

    let contract_info = ContractInputInfo {
        oracles: OracleInput {
            public_keys: vec![oracle.get_public_key()],
            event_id: event_id.clone(),
            threshold: 1,
        },
        contract_descriptor: descriptor,
    };

    let contract_input = ContractInput {
        offer_collateral: offer_collateral,
        accept_collateral: accept_collateral,
        maturity_time: maturity as u32,
        fee_rate: 2,
        contract_infos: vec![contract_info],
    };

    match &manager
        .lock()
        .unwrap_or_else(|e| {
            info!("--Recovering from poisoned thread in send_offer--");
            e.into_inner()
        })
        .send_offer(&contract_input, COUNTER_PARTY_PK.parse().unwrap())
    {
        Ok(dlc) => {
            debug!(
                "Create new offer dlc output: {}",
                serde_json::to_string(dlc).unwrap()
            );
            Response::json(dlc)
        }
        Err(e) => {
            info!("DLC manager - send offer error: {}", e.to_string());
            Response::json(&ErrorsResponse {
                status: 400,
                errors: vec![ErrorResponse {
                    message: e.to_string(),
                    code: None,
                }],
            })
            .with_status_code(400)
        }
    }
}

fn accept_offer(accept_dlc: AcceptDlc, manager: Arc<Mutex<DlcManager>>) -> Response {
    if let Some(Message::Sign(sign)) = match manager
        .lock()
        .unwrap_or_else(|e| {
            info!("--Recovering from poisoned thread in accept_offer--");
            e.into_inner()
        })
        .on_dlc_message(
            &Message::Accept(accept_dlc),
            COUNTER_PARTY_PK.parse().unwrap(),
        ) {
        Ok(dlc) => dlc,
        Err(e) => {
            info!("DLC manager - accept offer error: {}", e.to_string());
            return add_access_control_headers(
                Response::json(&ErrorsResponse {
                    status: 400,
                    errors: vec![ErrorResponse {
                        message: e.to_string(),
                        code: None,
                    }],
                })
                .with_status_code(400),
            );
        }
    } {
        debug!(
            "Accept offer - signed dlc output: {}",
            serde_json::to_string(&sign).unwrap()
        );
        add_access_control_headers(Response::json(&sign))
    } else {
        panic!();
    }
}

fn delete_all_offers(manager: Arc<Mutex<DlcManager>>, response: Response) -> Response {
    let man = manager.lock().unwrap();
    man.get_store().delete_contracts();
    return response;
}

fn add_access_control_headers(response: Response) -> Response {
    return response
        .with_additional_header("Access-Control-Allow-Origin", "*")
        .with_additional_header("Access-Control-Allow-Methods", "*")
        .with_additional_header("Access-Control-Allow-Headers", "*");
}
