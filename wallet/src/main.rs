// #![deny(warnings)]
#![feature(async_fn_in_trait)]
#![allow(unreachable_code)]

use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use bytes::Buf;
use tokio::sync::oneshot;

use hyper::service::{make_service_fn, service_fn};
use hyper::Error;
use hyper::{header, Body, Method, Response, Server, StatusCode};
use tokio::{task, time};
use url::form_urlencoded;

use bdk::{descriptor, FeeRate, SyncOptions};
use bdk::{SignOptions, Wallet as BdkWallet};
use serde::{Deserialize, Serialize};

use core::panic;
use std::time::Duration;
use std::{
    collections::HashMap,
    env,
    str::FromStr,
    sync::{Arc, Mutex},
};

use tokio::sync::Mutex as AsyncMutex;

use bitcoin::{Address, XOnlyPublicKey};
use dlc_bdk_wallet::DlcBdkWallet;
use dlc_link_manager::{AsyncOracle, AsyncStorage, Manager};
use dlc_manager::{
    contract::{
        contract_input::{ContractInput, ContractInputInfo, OracleInput},
        Contract,
    },
    SystemTimeProvider,
};
use dlc_messages::{AcceptDlc, Message};
use esplora_async_blockchain_provider::EsploraAsyncBlockchainProvider;
use tracing::{debug, error, info, warn};

use attestor_client::AttestorClient;
use dlc_clients::async_storage_provider::AsyncStorageApiProvider;
use serde_json::json;
use std::fmt::{self, Write as _};

use utils::get_numerical_contract_info;

mod utils;
#[macro_use]
mod macros;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
#[derive(Debug)]
struct WalletError(String);
impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wallet Error: {}", self.0)
    }
}
impl std::error::Error for WalletError {}
static NOTFOUND: &[u8] = b"Not Found";
type DlcManager<'a> = Manager<
    Arc<DlcBdkWallet>,
    Arc<EsploraAsyncBlockchainProvider>,
    Arc<AsyncStorageApiProvider>,
    Arc<AttestorClient>,
    Arc<SystemTimeProvider>,
>;

// The contracts in dlc-manager expect a node id, but web extensions often don't have this, so hardcode it for now. Should not have any ramifications.
const STATIC_COUNTERPARTY_NODE_ID: &str =
    "02fc8e97419286cf05e5d133f41ff6d51f691dda039e9dc007245a421e2c7ec61c";

const REQWEST_TIMEOUT: Duration = Duration::from_secs(30);

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

async fn get_attestors(
    blockchain_interface_url: String,
) -> Result<Vec<String>, dlc_manager::error::Error> {
    let get_all_attestors_endpoint_url = format!("{}/get-all-attestors", blockchain_interface_url);

    let res = reqwest::Client::new()
        .get(get_all_attestors_endpoint_url.as_str())
        .timeout(REQWEST_TIMEOUT)
        .send()
        .await
        .map_err(to_oracle_error)?;

    let attestors = res.json::<Vec<String>>().await.map_err(to_oracle_error)?;

    match attestors.len() {
        0 => Err(dlc_manager::error::Error::OracleError(
            "No attestors found".to_string(),
        )),
        _ => Ok(attestors),
    }
}

async fn generate_attestor_client(
    attestor_urls: Vec<String>,
) -> HashMap<XOnlyPublicKey, Arc<AttestorClient>> {
    let mut attestor_clients = HashMap::new();

    for url in attestor_urls.iter() {
        let p2p_client: AttestorClient = retry!(
            AttestorClient::new(url).await,
            10,
            "attestor client creation"
        );
        let attestor = Arc::new(p2p_client);
        attestor_clients.insert(attestor.get_public_key().await, attestor.clone());
    }
    attestor_clients
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let wallet_backend_port: String = env::var("WALLET_BACKEND_PORT").unwrap_or("8085".to_string());
    let bitcoin_check_interval_seconds: u64 = env::var("BITCOIN_CHECK_INTERVAL_SECONDS")
        .unwrap_or("10".to_string())
        .parse::<u64>()
        .unwrap_or(60);
    let local = task::LocalSet::new();
    local.spawn_local(async move {
        let mut interval =
            time::interval(time::Duration::from_secs(bitcoin_check_interval_seconds));
        loop {
            interval.tick().await;
            match reqwest::Client::new()
                .get(format!(
                    "http://localhost:{}/periodic_check",
                    wallet_backend_port
                ))
                .timeout(REQWEST_TIMEOUT)
                .send()
                .await
            {
                Ok(_) => (),
                Err(e) => {
                    warn!("Error running periodic check: {}, will retry", e);
                }
            }
        }
    });
    local.spawn_local(async move {
        run().await;
    });
    local.await;
}

fn build_success_response(message: String) -> Result<Response<Body>, GenericError> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_ALLOW_METHODS, "*")
        .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .body(Body::from(message.to_string()))
        .unwrap())
}

fn build_error_response(message: String) -> Result<Response<Body>, GenericError> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_ALLOW_METHODS, "*")
        .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!(
                {
                    "status": 400,
                    "errors": vec![ErrorResponse {
                        message: message.to_string(),
                        code: None,
                    }],
                }
            )
            .to_string(),
        ))?)
}

async fn run() {
    // Setup env vars
    let wallet_backend_port: String = env::var("WALLET_BACKEND_PORT").unwrap_or("8085".to_string());
    let xpriv_str = env::var("XPRIV")
        .expect("XPRIV environment variable not set, please run `just generate-key`, securely backup the output, and set this env_var accordingly");
    let xpriv = ExtendedPrivKey::from_str(&xpriv_str).expect("Unable to decode xpriv env variable");
    let fingerprint = env::var("FINGERPRINT")
        .expect("FINGERPRINT environment variable not set, please run `just generate-key`, securely backup the output, and set this env_var accordingly");
    if fingerprint
        != xpriv
            .fingerprint(&bitcoin::secp256k1::Secp256k1::new())
            .to_string()
    {
        error!("Fingerprint does not match xpriv fingerprint! Please make sure you have the correct xpriv and fingerprint set in your env variables\n\nExiting...");
        return;
    }
    let blockchain_interface_url = env::var("BLOCKCHAIN_INTERFACE_URL")
        .expect("BLOCKCHAIN_INTERFACE_URL environment variable not set, couldn't get attestors");
    let storage_api_url = env::var("STORAGE_API_ENDPOINT")
        .expect("STORAGE_API_ENDPOINT environment variable not set");
    let root_sled_path: String = env::var("SLED_WALLET_PATH").unwrap_or("wallet_db".to_string());
    let electrs_host =
        env::var("ELECTRUM_API_URL").expect("ELECTRUM_API_URL environment variable not set"); // Set up Blockchain Connection Object
    let active_network = match env::var("BITCOIN_NETWORK").as_deref() {
        Ok("bitcoin") => bitcoin::Network::Bitcoin,
        Ok("testnet") => bitcoin::Network::Testnet,
        Ok("signet") => bitcoin::Network::Signet,
        Ok("regtest") => bitcoin::Network::Regtest,
        _ => panic!(
            "Unknown Bitcoin Network, make sure to set BITCOIN_NETWORK in your env variables"
        ),
    };

    // Setup DLC UUID state tracking
    let funded_endpoint_url = format!("{}/set-status-funded", blockchain_interface_url);
    let closed_endpoint_url = format!("{}/post-close-dlc", blockchain_interface_url);
    let funded_uuids: Box<Vec<String>> = Box::default();
    let closed_uuids: Box<Vec<String>> = Box::default();

    // Set up wallet store
    let sled_path = format!("{root_sled_path}_{active_network}_{fingerprint}");
    let sled = sled::open(sled_path)
        .unwrap()
        .open_tree("default_tree")
        .unwrap();

    // ELECTRUM / ELECTRS
    let blockchain = Arc::new(EsploraAsyncBlockchainProvider::new(
        electrs_host.to_string(),
        active_network,
    ));
    let (pubkey_ext, wallet) = setup_wallets(xpriv, active_network, sled);

    // Do one initial sync of the wallet
    refresh_wallet(blockchain.clone(), wallet.clone())
        .await
        .unwrap();

    // Set up Attestor Clients
    let attestor_urls: Vec<String> = retry!(
        get_attestors(blockchain_interface_url.clone()).await,
        10,
        "Loading attestors from blockchain interface"
    );
    let protocol_wallet_attestors = generate_attestor_client(attestor_urls.clone()).await;

    retry!(
        blockchain.blockchain.get_height().await,
        10,
        "get blockchain height"
    );

    // Set up DLC store
    let dlc_store = Arc::new(AsyncStorageApiProvider::new(
        pubkey_ext.to_string(),
        storage_api_url,
    ));

    // Set up time provider
    let time_provider = SystemTimeProvider {};
    let manager = Arc::new(Mutex::new(
        Manager::new(
            Arc::clone(&wallet),
            Arc::clone(&blockchain),
            dlc_store.clone(),
            protocol_wallet_attestors.clone(),
            Arc::new(time_provider),
            // Arc::clone(&blockchain),
        )
        .unwrap(),
    ));

    let action = Arc::new(AsyncMutex::new("single_thread_lock"));
    let make_service = make_service_fn(move |_| {
        let action = action.clone();
        // For each connection, clone the counter to use in our service...
        let manager = manager.clone();
        let blockchain = blockchain.clone();
        let dlc_store = dlc_store.clone();
        let wallet = wallet.clone();
        let funded_endpoint_url = funded_endpoint_url.clone();
        let funded_uuids = funded_uuids.clone();
        let closed_endpoint_url = closed_endpoint_url.clone();
        let closed_uuids = closed_uuids.clone();

        async move {
            let action = action.clone();
            Ok::<_, Error>(service_fn(move |req| {
                let action = action.clone();
                let manager = manager.clone();
                let blockchain = blockchain.clone();
                let dlc_store = dlc_store.clone();
                let wallet = wallet.clone();
                let funded_endpoint_url = funded_endpoint_url.clone();
                let mut funded_uuids = funded_uuids.clone();
                let closed_endpoint_url = closed_endpoint_url.clone();
                let mut closed_uuids = closed_uuids.clone();
                async move {
                    // We currently lock the main process because of the various std::mutex calls inside
                    let _asdf = action.lock().await;
                    match (req.method(), req.uri().path()) {
                        (&Method::GET, "/empty_to_address") => {
                            let result = async {
                                let query = req.uri().query().ok_or(WalletError(
                                    "Unable to find query on Request object".to_string(),
                                ))?;
                                let params = form_urlencoded::parse(query.as_bytes())
                                    .into_owned()
                                    .collect::<HashMap<String, String>>();
                                let address = params.get("address").ok_or(WalletError(
                                    "Unable to find address in query params".to_string(),
                                ))?;
                                empty_to_address(address, wallet, blockchain).await
                            };
                            match result.await {
                                Ok(message) => build_success_response(message),
                                Err(e) => {
                                    warn!("Error emptying to address - {}", e);
                                    build_error_response(e.to_string())
                                }
                            }
                        }
                        (&Method::GET, "/info") => get_wallet_info(dlc_store, wallet).await,
                        (&Method::GET, "/periodic_check") => {
                            let result = async {
                                refresh_wallet(blockchain, wallet).await?;
                                periodic_check(
                                    manager,
                                    dlc_store,
                                    funded_endpoint_url,
                                    &mut funded_uuids,
                                    closed_endpoint_url,
                                    &mut closed_uuids,
                                )
                                .await
                            };
                            match result.await {
                                Ok(_) => (),
                                Err(e) => {
                                    warn!("Error periodic check: {}", e.to_string());
                                    return build_error_response(e.to_string());
                                }
                            };
                            build_success_response("Periodic check complete".to_string())
                        }
                        (&Method::OPTIONS, "/offer") => build_success_response("".to_string()),
                        (&Method::POST, "/offer") => {
                            #[derive(Deserialize)]
                            #[serde(rename_all = "camelCase")]
                            struct OfferRequest {
                                uuid: String,
                                accept_collateral: u64,
                                offer_collateral: u64,
                                total_outcomes: u64,
                                attestor_list: String,
                            }
                            let result = async {
                                let whole_body =
                                    hyper::body::aggregate(req).await.map_err(|e| {
                                        WalletError(format!("Error aggregating body: {}", e))
                                    })?;

                                let req: OfferRequest =
                                    serde_json::from_reader(whole_body.reader()).unwrap();

                                let bitcoin_contract_attestor_urls: Vec<String> =
                                    serde_json::from_str(&req.attestor_list.clone()).map_err(
                                        |e| {
                                            WalletError(format!(
                                                "Error deserializing attestor list: {}",
                                                e
                                            ))
                                        },
                                    )?;

                                let bitcoin_contract_attestors: HashMap<
                                    XOnlyPublicKey,
                                    Arc<AttestorClient>,
                                > = generate_attestor_client(
                                    bitcoin_contract_attestor_urls.clone(),
                                )
                                .await;

                                create_new_offer(
                                    manager,
                                    bitcoin_contract_attestors,
                                    active_network,
                                    req.uuid,
                                    req.accept_collateral,
                                    req.offer_collateral,
                                    req.total_outcomes,
                                )
                                .await
                            };
                            match result.await {
                                Ok(offer_message) => build_success_response(offer_message),
                                Err(e) => {
                                    warn!("Error generating offer - {}", e);
                                    build_error_response(e.to_string())
                                }
                            }
                        }
                        (&Method::OPTIONS, "/offer/accept") => {
                            build_success_response("".to_string())
                        }
                        (&Method::PUT, "/offer/accept") => {
                            info!("Accepting offer");
                            let result = async {
                                // Aggregate the body...
                                let whole_body = hyper::body::aggregate(req).await?;
                                // Decode as JSON...
                                #[derive(Deserialize)]
                                #[serde(rename_all = "camelCase")]
                                struct AcceptOfferRequest {
                                    accept_message: String,
                                }
                                let data: AcceptOfferRequest =
                                    serde_json::from_reader(whole_body.reader()).unwrap();
                                let accept_dlc: AcceptDlc =
                                    serde_json::from_str(&data.accept_message)?;
                                accept_offer(accept_dlc, manager).await
                            };
                            match result.await {
                                Ok(sign_message) => build_success_response(sign_message),
                                Err(e) => {
                                    warn!("Error accepting offer - {}", e);
                                    build_error_response(e.to_string())
                                }
                            }
                        }
                        _ => {
                            // Return 404 not found response.
                            Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(NOTFOUND.into())
                                .unwrap())
                        }
                    }
                }
            }))
        }
    });

    let addr = (
        [0, 0, 0, 0],
        wallet_backend_port.parse().expect("Correct port value"),
    )
        .into();

    let server = Server::bind(&addr).executor(LocalExec).serve(make_service);

    let (_tx, rx) = oneshot::channel::<()>();
    let server = server.with_graceful_shutdown(async move {
        rx.await.ok();
    });

    warn!("Listening on http://{}", addr);

    // The server would block on current thread to await !Send futures.
    if let Err(e) = server.await {
        panic!("server error: {}", e);
    }
}

// Since the Server needs to spawn some background tasks, we needed
// to configure an Executor that can spawn !Send futures...
#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
where
    F: std::future::Future + 'static, // not requiring `Send`
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}

fn setup_wallets(
    xpriv: ExtendedPrivKey,
    active_network: bitcoin::Network,
    sled: sled::Tree,
) -> (ExtendedPubKey, Arc<DlcBdkWallet>) {
    let secp = bitcoin::secp256k1::Secp256k1::new();

    let external_derivation_path =
        DerivationPath::from_str("m/44h/0h/0h/0").expect("A valid derivation path");

    let signing_external_descriptor = descriptor!(wpkh((
        xpriv,
        external_derivation_path.extend([ChildNumber::Normal { index: 0 }])
    )))
    .unwrap();

    let x = signing_external_descriptor.0.clone();

    let bdk_wallet = Arc::new(Mutex::new(
        BdkWallet::new(signing_external_descriptor, None, active_network, sled).unwrap(),
    ));

    let static_address = x.at_derivation_index(0).address(active_network).unwrap();
    let derived_ext_xpriv = xpriv
        .derive_priv(
            &secp,
            &external_derivation_path.extend([
                ChildNumber::Normal { index: 0 },
                ChildNumber::Normal { index: 0 },
            ]),
        )
        .unwrap();
    let seckey_ext = derived_ext_xpriv.private_key;

    let wallet: Arc<DlcBdkWallet> = Arc::new(DlcBdkWallet::new(
        bdk_wallet,
        static_address.clone(),
        seckey_ext,
        active_network,
    ));

    let pubkey = ExtendedPubKey::from_priv(&secp, &derived_ext_xpriv);
    (pubkey, wallet)
}

async fn create_new_offer(
    manager: Arc<Mutex<DlcManager<'_>>>,
    attestors: HashMap<XOnlyPublicKey, Arc<AttestorClient>>,
    active_network: bitcoin::Network,
    event_id: String,
    accept_collateral: u64,
    offer_collateral: u64,
    total_outcomes: u64,
) -> Result<String, WalletError> {
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

    let public_keys = attestors.clone().into_keys().collect();
    let contract_info = ContractInputInfo {
        oracles: OracleInput {
            public_keys,
            event_id: event_id.clone(),
            threshold: attestors.len() as u16,
        },
        contract_descriptor: descriptor,
    };

    // Some regtest networks have an unreliable fee estimation service
    let fee_rate = match active_network {
        bitcoin::Network::Regtest => 1,
        _ => 400,
    };

    let contract_input = ContractInput {
        offer_collateral,
        accept_collateral,
        fee_rate,
        contract_infos: vec![contract_info],
    };

    //had to make this mutable because of the borrow, not sure why
    let mut man = manager.lock().unwrap();

    let offer = man
        .send_offer(
            &contract_input,
            STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
        )
        .await
        .map_err(|e| WalletError(e.to_string()))?;
    serde_json::to_string(&offer).map_err(|e| WalletError(e.to_string()))
}

async fn accept_offer(
    accept_dlc: AcceptDlc,
    manager: Arc<Mutex<DlcManager<'_>>>,
) -> Result<String, GenericError> {
    let dlc = manager
        .lock()
        .unwrap()
        .on_dlc_message(
            &Message::Accept(accept_dlc),
            STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
        )
        .await?;

    match dlc {
        Some(Message::Sign(sign)) => serde_json::to_string(&sign).map_err(|e| e.into()),
        _ => Err("Error: invalid Sign message for accept_offer function".into()),
    }
}

async fn get_wallet_info(
    store: Arc<AsyncStorageApiProvider>,
    wallet: Arc<DlcBdkWallet>,
    // static_address: String,
) -> Result<Response<Body>, GenericError> {
    let mut info_response = json!({});
    let mut contracts_json = json!({});

    fn hex_str(value: &[u8]) -> String {
        let mut res = String::with_capacity(64);
        for v in value {
            write!(res, "{:02x}", v).unwrap();
        }
        res
    }

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
        .await
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
        "unconfirmed_balance": wallet.bdk_wallet.lock().unwrap().get_balance().unwrap().untrusted_pending,
        "balance": wallet.bdk_wallet.lock().unwrap().get_balance().unwrap().confirmed,
        "address": wallet.address
    });
    info_response["contracts"] = contracts_json;

    // Response::json(&info_response)
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(info_response.to_string()))?;
    Ok(response)
}

async fn refresh_wallet(
    blockchain: Arc<EsploraAsyncBlockchainProvider>,
    wallet: Arc<DlcBdkWallet>,
) -> Result<(), WalletError> {
    let bdk = match wallet.bdk_wallet.lock() {
        Ok(wallet) => wallet,
        Err(e) => {
            error!("Error locking wallet: {}", e.to_string());
            return Err(WalletError(e.to_string()));
        }
    };

    bdk.sync(&blockchain.blockchain, SyncOptions::default())
        .await
        .map_err(|e| WalletError(e.to_string()))?;

    debug!("BDK done syncing, now syncing blockchain");
    blockchain
        .refresh_chain_data(wallet.address.to_string())
        .await
        .map_err(|e| WalletError(e.to_string()))?;
    debug!("done syncing blockchain data");
    Ok(())
}

async fn periodic_check(
    manager: Arc<Mutex<DlcManager<'_>>>,
    store: Arc<AsyncStorageApiProvider>,
    funded_url: String,
    funded_uuids: &mut Vec<String>,
    closed_url: String,
    closed_uuids: &mut Vec<String>,
) -> Result<String, GenericError> {
    debug!("Running periodic_check");

    // This should ideally not be done as a mutable ref as it could cause a runtime error
    // when you have a reference to an object as mut and not mut at the same time
    let mut man = manager.lock().unwrap();

    let updated_contracts = match man.periodic_check().await {
        Ok(updated_contracts) => updated_contracts,
        Err(e) => {
            info!("Error in periodic_check, will retry: {}", e.to_string());
            vec![]
        }
    };
    let mut newly_confirmed_uuids: Vec<String> = vec![];
    let mut newly_closed_uuids: Vec<(String, bitcoin::Txid)> = vec![];

    for (id, uuid) in updated_contracts {
        let contract = match store.get_contract(&id).await {
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
            reqwest::Client::new()
                .post(&funded_url)
                .timeout(REQWEST_TIMEOUT)
                .json(&json!({ "uuid": uuid }))
                .send()
                .await?;
        }
    }

    for (uuid, txid) in newly_closed_uuids {
        if !closed_uuids.contains(&uuid) {
            debug!("Contract is closed, firing post-close url: {}", uuid);
            reqwest::Client::new()
                .post(&closed_url)
                .timeout(REQWEST_TIMEOUT)
                .json(&json!({"uuid": uuid, "btcTxId": txid.to_string()}))
                .send()
                .await?;
        }
    }
    Ok("Success running periodic check".to_string())
}

async fn empty_to_address(
    address: &str,
    wallet: Arc<DlcBdkWallet>,
    blockchain: Arc<EsploraAsyncBlockchainProvider>,
) -> Result<String, WalletError> {
    let bdk = match wallet.bdk_wallet.lock() {
        Ok(wallet) => wallet,
        Err(e) => {
            error!("Error locking wallet: {}", e.to_string());
            return Err(WalletError(e.to_string()));
        }
    };

    let to_address = Address::from_str(address).map_err(|e| WalletError(e.to_string()))?;
    info!("draining wallet to address: {}", to_address);
    let mut builder = bdk.build_tx();
    builder
        .drain_wallet()
        .drain_to(to_address.script_pubkey())
        .fee_rate(FeeRate::from_sat_per_vb(5.0))
        .enable_rbf();
    let (mut psbt, _details) = builder.finish().map_err(|e| WalletError(e.to_string()))?;

    let _finalized = bdk
        .sign(&mut psbt, SignOptions::default())
        .map_err(|e| WalletError(e.to_string()))?;

    // Broadcast the transaction
    let raw_transaction = psbt.extract_tx();
    let txid = raw_transaction.txid();

    blockchain
        .blockchain
        .broadcast(&raw_transaction)
        .await
        .map_err(|e| WalletError(e.to_string()))?;
    Ok(format!("Transaction broadcast successfully, TXID: {txid}"))
}
