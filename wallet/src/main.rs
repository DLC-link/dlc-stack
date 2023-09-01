#![feature(async_fn_in_trait)]
#![allow(unreachable_code)]
extern crate log;

#[macro_use]
extern crate rouille;

use std::{
    cmp,
    collections::HashMap,
    env,
    fs::File,
    io::{Read, Write},
    panic,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
    vec,
};

use bitcoin::{
    util::bip32::{DerivationPath, ExtendedPrivKey},
    Address, XOnlyPublicKey,
};
// use dlc_link_manager::Manager;
use dlc_manager::{
    contract::{
        contract_input::{ContractInput, ContractInputInfo, OracleInput},
        Contract,
    },
    manager::Manager,
    Blockchain, Oracle, Storage, SystemTimeProvider, Wallet,
};
use dlc_messages::{AcceptDlc, Message};
use dlc_simple_wallet::DlcSimpleWallet;
use dlc_sled_storage_provider::SledStorageProvider;
use electrs_blockchain_provider::ElectrsBlockchainProvider;
use log::{debug, error, info, warn};
// use secp256k1_zkp::{rand, All, PublicKey, Secp256k1, SecretKey};

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
    Arc<DlcSimpleWallet<Arc<ElectrsBlockchainProvider>, Arc<SledStorageProvider>>>,
    Arc<ElectrsBlockchainProvider>,
    Box<StorageProvider>,
    Arc<P2PDOracleClient>,
    Arc<SystemTimeProvider>,
    Arc<ElectrsBlockchainProvider>,
>;

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

pub fn to_oracle_error<T>(e: T) -> dlc_manager::error::Error
where
    T: std::fmt::Display,
{
    dlc_manager::error::Error::OracleError(e.to_string())
}

fn get_attestors() -> Result<Vec<String>, dlc_manager::error::Error> {
    let blockchain_interface_url = env::var("BLOCKCHAIN_INTERFACE_URL")
        .expect("BLOCKCHAIN_INTERFACE_URL environment variable not set, couldn't get attestors");

    let get_all_attestors_endpoint_url = format!("{}/get-all-attestors", blockchain_interface_url);

    let client = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .build()
        .map_err(to_oracle_error)?;

    let res = client
        .get(get_all_attestors_endpoint_url.as_str())
        .send()
        .map_err(to_oracle_error)?;

    let attestors = res.json::<Vec<String>>().map_err(to_oracle_error)?;

    match attestors.len() {
        0 => Err(dlc_manager::error::Error::OracleError(
            "No attestors found".to_string(),
        )),
        _ => Ok(attestors),
    }
}

// fn get_or_generate_secret_from_config(
//     secp: &Secp256k1<All>,
//     secret_key_file_path: std::path::PathBuf,
// ) -> SecretKey {
//     let mut secret_key = String::new();
//     if secret_key_file_path.exists() {
//         info!(
//             "reading secret key from {} (default)",
//             secret_key_file_path.file_name().unwrap().to_string_lossy()
//         );
//         File::open(secret_key_file_path)
//             .unwrap()
//             .read_to_string(&mut secret_key)
//             .unwrap();
//         secret_key.retain(|c| !c.is_whitespace());
//         SecretKey::from_str(&secret_key).unwrap()
//     } else {
//         info!("no secret key file was found, generating secret key");
//         let new_key = secp.generate_keypair(&mut rand::thread_rng()).0;
//         let mut file = File::create(secret_key_file_path).unwrap();
//         file.write_all(new_key.display_secret().to_string().as_bytes())
//             .unwrap();
//         new_key
//     }
// }

fn generate_p2pd_clients(
    attestor_urls: Vec<String>,
) -> HashMap<XOnlyPublicKey, Arc<P2PDOracleClient>> {
    let mut attestor_clients = HashMap::new();

    for url in attestor_urls.iter() {
        let p2p_client: P2PDOracleClient =
            retry!(P2PDOracleClient::new(url), 10, "oracle client creation");
        let attestor = Arc::new(p2p_client);
        attestor_clients.insert(attestor.get_public_key(), attestor.clone());
    }
    return attestor_clients;
}

fn main() {
    env_logger::init();
    let xpriv_str = env::var("XPRIV")
        .expect("XPRIV environment variable not set, please run `just generate-descriptor`, securely backup the output, and set this env_var accordingly");
    let xpriv = ExtendedPrivKey::from_str(&xpriv_str).expect("Unable to decode xpriv env variable");

    let attestor_urls: Vec<String> = retry!(get_attestors(), 10, "Loading attestors");

    let blockchain_interface_url = env::var("BLOCKCHAIN_INTERFACE_URL")
        .expect("BLOCKCHAIN_INTERFACE_URL environment variable not set, couldn't get attestors");

    let funded_endpoint_url = format!("{}/set-status-funded", blockchain_interface_url);
    let closed_endpoint_url = format!("{}/post-close-dlc", blockchain_interface_url);

    let wallet_backend_port: String = env::var("WALLET_BACKEND_PORT").unwrap_or("8085".to_string());
    let mut funded_uuids: Vec<String> = vec![];
    let mut closed_uuids: Vec<String> = vec![];

    // Setup Blockchain Connection Object
    let active_network = match env::var("BITCOIN_NETWORK").as_deref() {
        Ok("bitcoin") => bitcoin::Network::Bitcoin,
        Ok("testnet") => bitcoin::Network::Testnet,
        Ok("signet") => bitcoin::Network::Signet,
        Ok("regtest") => bitcoin::Network::Regtest,
        _ => panic!(
            "Unknown Bitcoin Network, make sure to set BITCOIN_NETWORK in your env variables"
        ),
    };

    // ELECTRUM / ELECTRS
    let electrs_host =
        env::var("ELECTRUM_API_URL").unwrap_or("https://blockstream.info/testnet/api/".to_string());
    let blockchain = Arc::new(ElectrsBlockchainProvider::new(
        electrs_host.to_string(),
        active_network,
    ));

    // Set up wallet store
    let root_sled_path: String = env::var("SLED_WALLET_PATH").unwrap_or("wallet_db".to_string());
    let sled_path = format!("{root_sled_path}_{}", active_network);
    let wallet_store: Arc<SledStorageProvider> =
        Arc::new(SledStorageProvider::new(sled_path.as_str()).unwrap());

    // Set up Oracle Client
    let protocol_wallet_attestors = generate_p2pd_clients(attestor_urls.clone());

    // Set up time provider
    let time_provider = SystemTimeProvider {};

    retry!(
        blockchain.get_blockchain_height(),
        10,
        "get blockchain height"
    );

    // let secret_key = get_or_generate_secret_from_config(&secp, PathBuf::from("secret.key"));
    let secp = bitcoin::secp256k1::Secp256k1::new();

    let external_path = DerivationPath::from_str("m/44h/0h/0h/0").expect("A valid derivation path");
    // let int_path = DerivationPath::from_str("m/44h/0h/0h/1").expect("A valid derivation path");

    let derived_external_xpriv = xpriv.derive_priv(&secp, &external_path).unwrap();
    let seckey_ext = derived_external_xpriv.private_key;
    let pubkey_raw = seckey_ext.public_key(&secp);
    // info!("pubkey_raw: {:?}", pubkey_raw.serialize());
    let pubkey = bitcoin::PublicKey {
        compressed: true,
        inner: pubkey_raw,
    };
    // info!("pubkey: {:?}", pubkey);

    // let pubkey = bitcoin::PublicKey::from_slice(&seckey_ext.public_key(&secp).serialize()).unwrap();

    // Set up wallet
    let wallet = Arc::new(DlcSimpleWallet::new(
        blockchain.clone(),
        wallet_store.clone(),
        active_network,
        Address::p2wpkh(&pubkey, active_network).unwrap(),
        seckey_ext,
    ));
    let static_address = wallet.get_new_address().unwrap();
    info!(
        "Starting Wallet with address: {}",
        static_address.to_string()
    );

    info!("Starting DLC Manager with pubkey: {}", pubkey.to_string());

    // Set up DLC store
    let dlc_store = StorageProvider::new(pubkey.to_string()).unwrap();

    // Create the DLC Manager
    let manager = Arc::new(Mutex::new(
        Manager::new(
            Arc::clone(&wallet),
            Arc::clone(&blockchain),
            Box::new(dlc_store),
            protocol_wallet_attestors.clone(),
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

    let manager2 = manager.clone();
    let wallet2 = wallet.clone();
    info!("Please query '/info' endpoint to get wallet info");
    info!("periodic_check loop thread starting");
    thread::spawn(move || loop {
        periodic_check(
            manager2.clone(),
            funded_endpoint_url.clone(),
            &mut funded_uuids,
            closed_endpoint_url.clone(),
            &mut closed_uuids,
        );
        wallet
            .refresh()
            .unwrap_or_else(|e| warn!("Error refreshing wallet {e}"));
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
                (GET) (/health) => {
                    Response::json(&("OK".to_string())).with_status_code(200)
                },
                (GET) (/unlockutxos) => {
                    unlock_utxos(wallet2.clone(), Response::json(&("OK".to_string())).with_status_code(200))
                },
                (GET) (/empty_to_address/{address: String}) => {
                    empty_to_address(address, wallet2.clone(), Response::json(&("OK".to_string())).with_status_code(200))
                },
                (GET) (/info) => {
                    info!("Call info.");
                    add_access_control_headers(get_wallet_info(manager.clone(), wallet2.clone(), static_address.to_string()))
                },
                (POST) (/offer) => {
                    info!("Call POST (create) offer {:?}", request);
                    #[derive(Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct OfferRequest {
                        uuid: String,
                        accept_collateral: u64,
                        offer_collateral: u64,
                        total_outcomes: u64,
                        attestor_list: String
                    }

                    let req: OfferRequest = try_or_400!(rouille::input::json_input(request));

                            // Set up Oracle Clients
                    let bitcoin_contract_attestor_urls: Vec<String> = match serde_json::from_str(&req.attestor_list.clone()) {
                        Ok(vec) => vec,
                        Err(e) => {
                            eprintln!("Error deserializing Attestor URLs: {}", e);
                            Vec::new()
                        }
                    };

                    let bitcoin_contract_attestors: HashMap<XOnlyPublicKey, Arc<P2PDOracleClient>> = generate_p2pd_clients(bitcoin_contract_attestor_urls.clone());

                    add_access_control_headers(create_new_offer(manager.clone(), bitcoin_contract_attestors.values().cloned().collect(), active_network, req.uuid, req.accept_collateral, req.offer_collateral, req.total_outcomes))
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

fn get_wallet_info(
    manager: Arc<Mutex<DlcManager>>,
    wallet: Arc<DlcSimpleWallet<Arc<ElectrsBlockchainProvider>, Arc<SledStorageProvider>>>,
    static_address: String,
) -> Response {
    let mut info_response = json!({});
    let mut contracts_json = json!({});

    fn hex_str(value: &[u8]) -> String {
        let mut res = String::with_capacity(64);
        for v in value {
            write!(res, "{:02x}", v).unwrap();
        }
        res
    }

    let man = manager.lock().unwrap();
    let store = man.get_store();

    let mut collected_contracts: Vec<Vec<String>> = vec![
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    ];

    let contracts = store
        .get_contracts()
        .expect("Error retrieving contract list.");

    for contract in contracts {
        let id = hex_str(&contract.get_id());
        match contract {
            Contract::Offered(_) => {
                collected_contracts[0].push(id);
            }
            Contract::Accepted(_) => {
                collected_contracts[1].push(id);
            }
            Contract::Confirmed(_) => {
                collected_contracts[2].push(id);
            }
            Contract::Signed(_) => {
                collected_contracts[3].push(id);
            }
            Contract::Closed(_) => {
                collected_contracts[4].push(id);
            }
            Contract::Refunded(_) => {
                collected_contracts[5].push(id);
            }
            Contract::FailedAccept(_) | Contract::FailedSign(_) => {
                collected_contracts[6].push(id);
            }
            Contract::Rejected(_) => collected_contracts[7].push(id),
            Contract::PreClosed(_) => collected_contracts[8].push(id),
        }
    }

    contracts_json["Offered"] = collected_contracts[0].clone().into();
    contracts_json["Accepted"] = collected_contracts[1].clone().into();
    contracts_json["Confirmed"] = collected_contracts[2].clone().into();
    contracts_json["Signed"] = collected_contracts[3].clone().into();
    contracts_json["Closed"] = collected_contracts[4].clone().into();
    contracts_json["Refunded"] = collected_contracts[5].clone().into();
    contracts_json["Failed"] = collected_contracts[6].clone().into();
    contracts_json["Rejected"] = collected_contracts[7].clone().into();
    contracts_json["PreClosed"] = collected_contracts[8].clone().into();

    info_response["wallet"] = json!({
        "balance": wallet.get_balance(),
        "address": static_address
    });
    info_response["contracts"] = contracts_json;

    Response::json(&info_response)
}

fn periodic_check(
    manager: Arc<Mutex<DlcManager>>,
    funded_url: String,
    funded_uuids: &mut Vec<String>,
    closed_url: String,
    closed_uuids: &mut Vec<String>,
) -> () {
    let mut man = manager.lock().unwrap();

    let updated_contracts = match man.periodic_check() {
        Ok(updated_contracts) => updated_contracts,
        Err(e) => {
            info!("Error in periodic_check, will retry: {}", e.to_string());
            vec![]
        }
    };

    let store = man.get_store();

    let mut newly_confirmed_uuids: Vec<String> = vec![];
    let mut newly_closed_uuids: Vec<(String, bitcoin::Txid)> = vec![];

    for (id, uuid) in updated_contracts {
        let contract = match store.get_contract(&id) {
            Ok(Some(contract)) => contract,
            Ok(None) => {
                error!("Error retrieving contract: {:?}", id);
                continue;
            }
            Err(e) => {
                error!("Error retrieving contract: {}", e.to_string());
                continue;
            }
        };

        match contract {
            Contract::Confirmed(_c) => {
                newly_confirmed_uuids.push(uuid);
            }
            Contract::Closed(c) => {
                newly_closed_uuids.push((uuid, c.signed_cet.unwrap().txid()));
            }
            _ => error!(
                "Error retrieving contract in periodic_check: {:?}, skipping",
                id
            ),
        };
    }

    for uuid in newly_confirmed_uuids {
        if !funded_uuids.contains(&uuid) {
            debug!("Contract is funded, setting funded to true: {}", uuid);
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
                            info!("Error setting funded to true: {}: {}", uuid, e.to_string());
                        }
                    },
                    Err(e) => {
                        info!("Error setting funded to true: {}: {}", uuid, e.to_string());
                    }
                }
            }
        }
    }

    for (uuid, txid) in newly_closed_uuids {
        if !closed_uuids.contains(&uuid) {
            debug!("Contract is closed, firing post-close url: {}", uuid);
            let mut post_body = HashMap::new();
            let txid = txid.to_string();
            post_body.insert("uuid", &uuid);
            post_body.insert("btcTxId", &txid);

            let client = reqwest::blocking::Client::builder()
                .use_rustls_tls()
                .build();
            if client.is_ok() {
                let res = client.unwrap().post(&closed_url).json(&post_body).send();

                match res {
                    Ok(res) => match res.error_for_status() {
                        Ok(_res) => {
                            closed_uuids.push(uuid.clone());
                            info!(
                                "Success setting contract to closed: {}, {}",
                                uuid,
                                _res.status()
                            );
                        }
                        Err(e) => {
                            info!(
                                "Error setting contract to closed: {}: {}",
                                uuid,
                                e.to_string()
                            );
                        }
                    },
                    Err(e) => {
                        info!(
                            "Error setting contract to closed: {}: {}",
                            uuid,
                            e.to_string()
                        );
                    }
                }
            }
        }
    }
}

fn create_new_offer(
    manager: Arc<Mutex<DlcManager>>,
    attestors: Vec<Arc<P2PDOracleClient>>,
    active_network: bitcoin::Network,
    event_id: String,
    accept_collateral: u64,
    offer_collateral: u64,
    total_outcomes: u64,
) -> Response {
    let (_event_descriptor, descriptor) = get_numerical_contract_info(
        accept_collateral,
        offer_collateral,
        total_outcomes,
        attestors.len(),
    );
    info!(
        "Creating new offer with event id: {}, accept collateral: {}, offer_collateral: {}",
        event_id.clone(),
        accept_collateral,
        offer_collateral
    );

    let contract_info = ContractInputInfo {
        oracles: OracleInput {
            public_keys: attestors.iter().map(|o| o.get_public_key()).collect(),
            event_id: event_id.clone(),
            threshold: attestors.len() as u16,
        },
        contract_descriptor: descriptor,
    };

    for attestor in attestors {
        // check if the oracle has an event with the id of event_id
        match attestor.get_announcement(&event_id) {
            Ok(_announcement) => (),
            Err(e) => {
                info!("Error getting announcement: {}", event_id);
                return Response::json(
                    &(ErrorsResponse {
                        status: 400,
                        errors: vec![ErrorResponse {
                            message: format!(
                                "Error: unable to get announcement. Does it exist? -- {}",
                                e.to_string()
                            ),
                            code: None,
                        }],
                    }),
                )
                .with_status_code(400);
            }
        }
    }

    // Some regtest networks have an unreliable fee estimation service
    let fee_rate = match active_network {
        bitcoin::Network::Regtest => 1,
        _ => 400,
    };

    println!("contract_info: {:?}", contract_info);

    let contract_input = ContractInput {
        offer_collateral: offer_collateral,
        accept_collateral: accept_collateral,
        fee_rate,
        contract_infos: vec![contract_info],
    };

    match &manager.lock().unwrap().send_offer(
        &contract_input,
        STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
    ) {
        Ok(dlc) => Response::json(&(dlc)),
        Err(e) => {
            info!("DLC manager - send offer error: {}", e.to_string());
            Response::json(
                &(ErrorsResponse {
                    status: 400,
                    errors: vec![ErrorResponse {
                        message: e.to_string(),
                        code: None,
                    }],
                }),
            )
            .with_status_code(400)
        }
    }
}

fn accept_offer(accept_dlc: AcceptDlc, manager: Arc<Mutex<DlcManager>>) -> Response {
    println!("accept_dlc: {:?}", accept_dlc);
    if let Some(Message::Sign(sign)) = match manager.lock().unwrap().on_dlc_message(
        &Message::Accept(accept_dlc),
        STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
    ) {
        Ok(dlc) => dlc,
        Err(e) => {
            info!("DLC manager - accept offer error: {}", e.to_string());
            return add_access_control_headers(
                Response::json(
                    &(ErrorsResponse {
                        status: 400,
                        errors: vec![ErrorResponse {
                            message: e.to_string(),
                            code: None,
                        }],
                    }),
                )
                .with_status_code(400),
            );
        }
    } {
        add_access_control_headers(Response::json(&sign))
    } else {
        return Response::json(
            &(ErrorsResponse {
                status: 400,
                errors: vec![ErrorResponse {
                    message: format!("Error: invalid Sign message for accept_offer function"),
                    code: None,
                }],
            }),
        )
        .with_status_code(400);
    }
}

fn delete_all_offers(manager: Arc<Mutex<DlcManager>>, response: Response) -> Response {
    info!("Deleting all contracts from dlc-store");
    let man = manager.lock().unwrap();
    man.get_store().delete_contracts();
    return response;
}

fn unlock_utxos(
    wallet: Arc<DlcSimpleWallet<Arc<ElectrsBlockchainProvider>, Arc<SledStorageProvider>>>,
    response: Response,
) -> Response {
    info!("Unlocking UTXOs");
    wallet.unreserve_all_utxos();
    return response;
}

fn empty_to_address(
    address: String,
    wallet: Arc<DlcSimpleWallet<Arc<ElectrsBlockchainProvider>, Arc<SledStorageProvider>>>,
    response: Response,
) -> Response {
    info!("Unlocking UTXOs");
    match wallet.empty_to_address(&Address::from_str(&address).unwrap()) {
        Ok(_) => info!("Emptied bitcoin to {address}"),
        Err(_) => warn!("Failed emptying bitcoin to {address}"),
    }
    return response;
}

fn add_access_control_headers(response: Response) -> Response {
    return response
        .with_additional_header("Access-Control-Allow-Origin", "*")
        .with_additional_header("Access-Control-Allow-Methods", "*")
        .with_additional_header("Access-Control-Allow-Headers", "*");
}
