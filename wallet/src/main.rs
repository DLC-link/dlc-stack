#![allow(unreachable_code)]
extern crate log;

#[macro_use]
extern crate rouille;

use std::{
    cmp,
    collections::HashMap,
    env, panic,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    vec,
};

use blockcypher_blockchain_provider::BlockcypherBlockchainProvider;
use dlc_manager::{
    contract::{
        contract_input::{ContractInput, ContractInputInfo, OracleInput},
        Contract,
    },
    manager::Manager,
    Blockchain, Oracle, Storage, SystemTimeProvider, Wallet,
};
use dlc_messages::{AcceptDlc, Message};
use dlc_sled_storage_provider::SledStorageProvider;
use electrs_blockchain_provider::ElectrsBlockchainProvider;
use log::{debug, info, warn};
use simple_wallet::SimpleWallet;

use crate::storage::storage_provider::StorageProvider;
use oracle_client::P2PDOracleClient;
use rouille::Response;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Write as _;
use utils::get_numerical_contract_info;

mod oracle_client;
mod storage;
mod utils;
#[macro_use]
mod macros;

type DlcManager<'a> = Manager<
    Arc<SimpleWallet<Arc<BlockcypherBlockchainProvider>, Arc<SledStorageProvider>>>,
    Arc<BlockcypherBlockchainProvider>,
    Box<StorageProvider>,
    Arc<P2PDOracleClient>,
    Arc<SystemTimeProvider>,
    Arc<BlockcypherBlockchainProvider>,
>;

const NUM_CONFIRMATIONS: u32 = 2;

// The contracts in dlc-manager expect a node id, but web extensions often don't have this, so hardcode it for now. Should not have any ramifications.
const STATIC_COUNTERPARTY_NODE_ID: &str =
    "02fc8e97419286cf05e5d133f41ff6d51f691dda039e9dc007245a421e2c7ec61c";

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
    let oracle_url: String = env::var("ORACLE_URL").unwrap_or("http://localhost:8080".to_string());

    let funded_url: String = env::var("FUNDED_URL")
        .unwrap_or("https://stacks-observer-mocknet.herokuapp.com/funded".to_string());
    let wallet_backend_port: String = env::var("WALLET_BACKEND_PORT").unwrap_or("8085".to_string());
    let mut funded_uuids: Vec<String> = vec![];

    // Setup Blockchain Connection Object
    // ELECTRUM / ELECTRS
    // let electrs_host =
    //     env::var("ELECTRUM_API_URL").unwrap_or("https://blockstream.info/testnet/api/".to_string());
    // let blockchain = Arc::new(ElectrsBlockchainProvider::new(
    //     electrs_host.to_string(),
    //     bitcoin::Network::Testnet,
    // ));

    // Blockcypher
    let blockcypher_host =
        env::var("BLOCKCYPHER_API_URL").unwrap_or("https://api.blockcypher.com/".to_string());
    let blockchain = Arc::new(BlockcypherBlockchainProvider::new(
        blockcypher_host.to_string(),
        bitcoin::Network::Regtest,
    ));

    // Set up DLC store
    let store = StorageProvider::new();

    // Set up wallet store
    let sled_path: String = env::var("SLED_PATH").unwrap_or("wallet_db".to_string());
    let wallet_store = Arc::new(SledStorageProvider::new(sled_path.as_str()).unwrap());

    // Set up wallet
    let wallet = Arc::new(SimpleWallet::new(
        blockchain.clone(),
        wallet_store.clone(),
        bitcoin::Network::Testnet,
    ));

    // Set up Oracle Client
    let p2p_client: P2PDOracleClient = retry!(
        P2PDOracleClient::new(&oracle_url),
        10,
        "oracle client creation"
    );
    let oracle = Arc::new(p2p_client);
    let oracles: HashMap<bitcoin::XOnlyPublicKey, _> =
        HashMap::from([(oracle.get_public_key(), oracle.clone())]);

    // Set up time provider
    let time_provider = SystemTimeProvider {};

    // Create the DLC Manager
    let manager = Arc::new(Mutex::new(
        Manager::new(
            Arc::clone(&wallet),
            Arc::clone(&blockchain),
            Box::new(store),
            oracles,
            Arc::new(time_provider),
            Arc::clone(&blockchain),
        )
        .unwrap(),
    ));

    // Start periodic_check thread

    let bitcoin_check_interval_seconds: u64 = env::var("BITCOIN_CHECK_INTERVAL_SECONDS")
        .unwrap_or("10".to_string())
        .parse::<u64>()
        .unwrap_or(10);
    let man2 = manager.clone();
    info!("periodic_check loop thread starting");
    debug!("Wallet address: {:?}", wallet.get_new_address());
    thread::spawn(move || loop {
        periodic_check(
            man2.clone(),
            blockchain.clone(),
            funded_url.clone(),
            &mut funded_uuids,
        );
        debug!("Wallet balance: {}", wallet.get_balance());
        wallet
            .refresh() //Do I really need to call this every 10 seconds?
            .unwrap_or_else(|e| println!("Error refreshing wallet {e}"));
        thread::sleep(Duration::from_millis(
            cmp::max(10, bitcoin_check_interval_seconds) * 1000,
        ));
    });

    rouille::start_server(format!("0.0.0.0:{}", wallet_backend_port), move |request| {
        router!(request,
                (GET) (/cleanup) => {
                    let contract_cleanup_enabled: bool = env::var("CONTRACT_CLEANUP_ENABLED")
                        .unwrap_or("false".to_string())
                        .parse().unwrap_or(false);
                    if contract_cleanup_enabled {
                        info!("Call cleanup contract offers.");
                        delete_all_offers(manager.clone(), Response::json(&("OK".to_string())).with_status_code(200))
                    } else {
                        info!("Call cleanup contract offers feature disabled.");
                        Response::json(&("Disabled".to_string())).with_status_code(400)
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

fn periodic_check(
    manager: Arc<Mutex<DlcManager>>,
    blockchain: Arc<dyn Blockchain>,
    funded_url: String,
    funded_uuids: &mut Vec<String>,
) -> Response {
    let mut collected_response = json!({});
    let mut man = manager.lock().unwrap();

    match man.periodic_check() {
        Ok(_) => (),
        Err(e) => {
            info!("Error in periodic_check, will retry: {}", e.to_string());
            return Response::empty_400();
        }
    };

    let store = man.get_store();

    collected_response["signed_contracts"] = store
        .get_signed_contracts()
        .unwrap_or(vec![])
        .iter()
        .map(|c| {
            let confirmations = match blockchain
                .get_transaction_confirmations(&c.accepted_contract.dlc_transactions.fund.txid())
            {
                Ok(confirms) => confirms,
                Err(e) => {
                    info!("Error checking confirmations: {}", e.to_string());
                    0
                }
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

                    let client = reqwest::blocking::Client::builder()
                        .use_rustls_tls()
                        .build();
                    if client.is_ok() {
                        let res = client.unwrap().post(&funded_url).json(&post_body).send();

                        match res {
                            Ok(res) => match res.error_for_status() {
                                Ok(_res) => {
                                    funded_uuids.push(uuid.clone());
                                    info!(
                                        "Success setting funded to true: {}, {}",
                                        uuid,
                                        _res.status()
                                    );
                                }
                                Err(e) => {
                                    info!(
                                        "Error setting funded to true: {}: {}",
                                        uuid,
                                        e.to_string()
                                    );
                                }
                            },
                            Err(e) => {
                                info!("Error setting funded to true: {}: {}", uuid, e.to_string());
                            }
                        }
                    }
                }
            }
            c.accepted_contract.get_contract_id_string()
        })
        .collect();

    collected_response["confirmed_contracts"] = store
        .get_confirmed_contracts()
        .unwrap_or(vec![])
        .iter()
        .map(|c| c.accepted_contract.get_contract_id_string())
        .collect();

    collected_response["preclosed_contracts"] = store
        .get_preclosed_contracts()
        .unwrap_or(vec![])
        .iter()
        .map(|c| c.signed_contract.accepted_contract.get_contract_id_string())
        .collect();

    let mut closed_contracts: Vec<String> = Vec::new();
    for val in store.get_contracts().unwrap_or(vec![]).iter() {
        if let Contract::Closed(c) = val {
            let mut string_id = String::with_capacity(32 * 2 + 2);
            string_id.push_str("0x");
            for i in &c.contract_id {
                write!(string_id, "{:02x}", i).unwrap();
            }
            closed_contracts.push(string_id);
        }
    }
    collected_response["closed_contracts"] = closed_contracts.into();

    debug!("check_close collected_response: {}", collected_response);
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
    info!(
        "Creating new offer with event id: {}, accept collateral: {}, offer_collateral: {}",
        event_id.clone(),
        accept_collateral,
        offer_collateral
    );

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
        fee_rate: 2,
        contract_infos: vec![contract_info],
    };

    match &manager.lock().unwrap().send_offer(
        &contract_input,
        STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
    ) {
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
    if let Some(Message::Sign(sign)) = match manager.lock().unwrap().on_dlc_message(
        &Message::Accept(accept_dlc),
        STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
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
