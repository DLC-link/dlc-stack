#![feature(async_fn_in_trait)]
#![allow(unreachable_code)]
extern crate console_error_panic_hook;
extern crate log;

use bitcoin::{Network, PrivateKey, XOnlyPublicKey};
use dlc_link_manager::AsyncOracle;
use dlc_messages::{Message, OfferDlc, SignDlc};
use secp256k1_zkp::UpstreamError;
use wasm_bindgen::prelude::*;

use lightning::util::ser::Readable;

use secp256k1_zkp::hashes::*;
use secp256k1_zkp::Secp256k1;

use core::panic;
use std::fmt;
use std::{
    collections::HashMap,
    io::Cursor,
    str::FromStr,
    sync::{Arc, Mutex},
};

use dlc_manager::{
    contract::{signed_contract::SignedContract, Contract},
    ContractId, SystemTimeProvider,
};

use dlc_link_manager::{AsyncStorage, Manager};

use std::fmt::Write as _;

use dlc_clients::async_storage_provider::AsyncStorageApiProvider;

use esplora_async_blockchain_provider_js_wallet::EsploraAsyncBlockchainProviderJsWallet;

use js_interface_wallet::JSInterfaceWallet;

use attestor_client::AttestorClient;
use serde::{Deserialize, Serialize};

#[macro_use]
mod macros;

#[derive(Debug)]
struct WalletError(String);
impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wallet Error: {}", self.0)
    }
}
impl std::error::Error for WalletError {}

async fn generate_attestor_client(
    attestor_urls: Vec<String>,
) -> HashMap<XOnlyPublicKey, Arc<AttestorClient>> {
    let mut attestor_clients = HashMap::new();

    for url in attestor_urls.iter() {
        let p2p_client: AttestorClient = AttestorClient::new(url).await.unwrap();
        let attestor = Arc::new(p2p_client);
        attestor_clients.insert(attestor.get_public_key().await, attestor.clone());
    }
    attestor_clients
}

type DlcManager = Manager<
    Arc<JSInterfaceWallet>,
    Arc<EsploraAsyncBlockchainProviderJsWallet>,
    Box<AsyncStorageApiProvider>,
    Arc<AttestorClient>,
    Arc<SystemTimeProvider>,
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

#[derive(Serialize, Deserialize)]
struct UtxoInput {
    txid: String,
    vout: u32,
    value: u64,
}

#[wasm_bindgen]
pub struct JsDLCInterface {
    options: JsDLCInterfaceOptions,
    manager: Arc<Mutex<DlcManager>>,
    wallet: Arc<JSInterfaceWallet>,
    blockchain: Arc<EsploraAsyncBlockchainProviderJsWallet>,
}

// #[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsDLCInterfaceOptions {
    attestor_urls: String,
    network: String,
    electrs_url: String,
    address: String,
}

impl Default for JsDLCInterfaceOptions {
    // Default values for Manager Options
    fn default() -> Self {
        Self {
            attestor_urls: "https://devnet.dlc.link/oracle".to_string(),
            network: "regtest".to_string(),
            electrs_url: "https://devnet.dlc.link/electrs".to_string(),
            address: "".to_string(),
        }
    }
}

#[wasm_bindgen]
impl JsDLCInterface {
    pub async fn new(
        privkey: String,
        address: String,
        network: String,
        electrs_url: String,
        attestor_urls: String,
    ) -> JsDLCInterface {
        console_error_panic_hook::set_once();

        let options = JsDLCInterfaceOptions {
            attestor_urls,
            network,
            electrs_url,
            address,
        };

        let active_network: Network = options
            .network
            .parse::<Network>()
            .expect("Must use a valid bitcoin network");

        let blockchain: Arc<EsploraAsyncBlockchainProviderJsWallet> =
            Arc::new(EsploraAsyncBlockchainProviderJsWallet::new(
                options.electrs_url.to_string(),
                active_network,
            ));

        // Generate keypair from secret key
        let seckey = secp256k1_zkp::SecretKey::from_str(&privkey).unwrap();

        let secp = Secp256k1::new();

        // let pubkey = PublicKey::from_secret_key(&secp, &seckey);
        let pubkey =
            bitcoin::PublicKey::from_private_key(&secp, &PrivateKey::new(seckey, active_network));

        // Set up DLC store
        let dlc_store = AsyncStorageApiProvider::new(
            pubkey.to_string(),
            "https://devnet.dlc.link/storage-api".to_string(),
        );

        // Set up wallet
        let wallet = Arc::new(JSInterfaceWallet::new(
            options.address.to_string(),
            PrivateKey::new(seckey, active_network),
        ));

        // Set up Oracle Clients
        let attestor_urls_vec: Vec<String> =
            match serde_json::from_str(&options.attestor_urls.clone()) {
                Ok(vec) => vec,
                Err(e) => {
                    eprintln!("Error deserializing Attestor URLs: {}", e);
                    Vec::new()
                }
            };

        let attestors = generate_attestor_client(attestor_urls_vec).await;

        // Set up time provider
        let time_provider = SystemTimeProvider {};

        // Create the DLC Manager
        let manager = Arc::new(Mutex::new(
            Manager::new(
                Arc::clone(&wallet),
                Arc::clone(&blockchain),
                Box::new(dlc_store),
                attestors,
                Arc::new(time_provider),
            )
            .unwrap(),
        ));

        match blockchain.refresh_chain_data(options.address.clone()).await {
            Ok(_) => (),
            Err(e) => {
                log_to_console!("Error refreshing chain data: {}", e);
            }
        };

        JsDLCInterface {
            options,
            manager,
            wallet,
            blockchain,
        }
    }

    pub fn get_options(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.options).unwrap()
    }

    pub async fn get_wallet_balance(&self) -> u64 {
        log_to_console!("get_wallet_balance");
        match self
            .blockchain
            .refresh_chain_data(self.options.address.clone())
            .await
        {
            Ok(_) => (),
            Err(e) => {
                log_to_console!("Error refreshing chain data: {}", e);
            }
        };
        match self.wallet.set_utxos(self.blockchain.get_utxos().unwrap()) {
            Ok(_) => (),
            Err(e) => {
                log_to_console!("Error setting utxos: {}", e);
            }
        };
        match self.blockchain.get_balance().await {
            Ok(balance) => balance,
            Err(e) => {
                log_to_console!("Error getting balance: {}", e);
                0
            }
        }
    }

    // public async function for fetching all the contracts on the manager
    pub async fn get_contracts(&self) -> JsValue {
        let contracts: Vec<JsContract> = self
            .manager
            .lock()
            .unwrap()
            .get_store()
            .get_contracts()
            .await
            .unwrap()
            .into_iter()
            .map(|contract| JsContract::from_contract(contract))
            .collect();

        serde_wasm_bindgen::to_value(&contracts).unwrap()
    }

    // public async function for fetching one contract as a JsContract type
    pub async fn get_contract(&self, contract_str: String) -> JsValue {
        let contract_id = ContractId::read(&mut Cursor::new(&contract_str)).unwrap();
        let contract = self
            .manager
            .lock()
            .unwrap()
            .get_store()
            .get_contract(&contract_id)
            .await
            .unwrap();
        match contract {
            Some(contract) => {
                serde_wasm_bindgen::to_value(&JsContract::from_contract(contract)).unwrap()
            }
            None => JsValue::NULL,
        }
    }

    pub async fn accept_offer(&self, offer_json: String) -> String {
        //could consider doing a refresh_chain_data here to have the newest utxos

        let accept_msg_result = async {
            let dlc_offer_message: OfferDlc =
                serde_json::from_str(&offer_json).map_err(|e| WalletError(e.to_string()))?;
            let temporary_contract_id = dlc_offer_message.temporary_contract_id;

            let counterparty = STATIC_COUNTERPARTY_NODE_ID
                .parse()
                .map_err(|e: UpstreamError| WalletError(e.to_string()))?;
            self.manager
                .lock()
                .unwrap()
                .on_dlc_message(&Message::Offer(dlc_offer_message.clone()), counterparty)
                .await
                .map_err(|e| WalletError(e.to_string()))?;
            let (_contract_id, _public_key, accept_msg) = self
                .manager
                .lock()
                .unwrap()
                .accept_contract_offer(&temporary_contract_id)
                .await
                .expect("Error accepting contract offer");
            serde_json::to_string(&accept_msg).map_err(|e| WalletError(e.to_string()))
        }
        .await;
        match accept_msg_result {
            Ok(accept_msg) => accept_msg,
            Err(e) => {
                log_to_console!("Error accepting offer: {}", e);
                format!("Error accepting offer: {}", e)
            }
        }
    }

    pub async fn countersign_and_broadcast(&self, dlc_sign_message: String) -> String {
        let dlc_sign_message: SignDlc = serde_json::from_str(&dlc_sign_message).unwrap();
        match self
            .manager
            .lock()
            .unwrap()
            .on_dlc_message(
                &Message::Sign(dlc_sign_message.clone()),
                STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
            )
            .await
        {
            Ok(_) => (),
            Err(e) => {
                log_to_console!("DLC manager - sign offer error: {}", e.to_string());
                panic!();
            }
        }
        let manager = self.manager.lock().unwrap();
        let store = manager.get_store();
        let contract: SignedContract = store
            .get_signed_contracts()
            .await
            .unwrap()
            .into_iter()
            .filter(|c| c.accepted_contract.get_contract_id() == dlc_sign_message.contract_id)
            .next()
            .unwrap();
        contract
            .accepted_contract
            .dlc_transactions
            .fund
            .txid()
            .to_string()
    }

    pub async fn reject_offer(&self, contract_id: String) -> () {
        let contract_id = ContractId::read(&mut Cursor::new(&contract_id)).unwrap();
        let contract = self
            .manager
            .lock()
            .unwrap()
            .get_store()
            .get_contract(&contract_id)
            .await
            .unwrap();

        match contract {
            Some(Contract::Offered(c)) => {
                self.manager
                    .lock()
                    .unwrap()
                    .get_store()
                    .update_contract(&Contract::Rejected(c))
                    .await
                    .unwrap();
            }
            _ => (),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[wasm_bindgen]
#[serde(rename_all = "camelCase")]
struct JsContract {
    id: String,
    state: String,
    acceptor_collateral: String,
    tx_id: String,
}

// implement the from_contract method for JsContract
impl JsContract {
    fn from_contract(contract: Contract) -> JsContract {
        let state = match contract.clone() {
            Contract::Offered(_) => "Offered",
            Contract::Accepted(_) => "Accepted",
            Contract::Signed(_) => "Signed",
            Contract::Confirmed(_) => "Confirmed",
            Contract::PreClosed(_) => "Pre-Closed",
            Contract::Closed(_) => "Closed",
            Contract::Refunded(_) => "Refunded",
            Contract::FailedAccept(_) => "Accept Failed",
            Contract::FailedSign(_) => "Sign Failed",
            Contract::Rejected(_) => "Rejected",
        };

        let acceptor_collateral: String = match contract.clone() {
            Contract::Accepted(c) => c.accept_params.collateral.to_string(),
            Contract::Signed(c) | Contract::Confirmed(c) | Contract::Refunded(c) => {
                c.accepted_contract.accept_params.collateral.to_string()
            }
            Contract::FailedSign(c) => c.accepted_contract.accept_params.collateral.to_string(),
            Contract::PreClosed(c) => c
                .signed_contract
                .accepted_contract
                .accept_params
                .collateral
                .to_string(),
            _ => String::new(),
        };

        let tx_id: String = match contract.clone() {
            Contract::Accepted(c) => c.dlc_transactions.fund.txid().to_string(),
            Contract::Signed(c) | Contract::Confirmed(c) | Contract::Refunded(c) => {
                c.accepted_contract.dlc_transactions.fund.txid().to_string()
            }
            Contract::FailedSign(c) => c.accepted_contract.dlc_transactions.fund.txid().to_string(),
            Contract::PreClosed(c) => c
                .signed_contract
                .accepted_contract
                .accept_params
                .collateral
                .to_string(),
            _ => String::new(),
        };

        fn hex_str(value: &[u8]) -> String {
            let mut res = String::with_capacity(64);
            for v in value {
                write!(res, "{:02x}", v).unwrap();
            }
            res
        }

        JsContract {
            id: hex_str(&contract.get_id()),
            state: state.to_string(),
            acceptor_collateral,
            tx_id,
        }
    }
}
