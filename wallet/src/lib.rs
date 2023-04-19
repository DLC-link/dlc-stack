#![allow(unreachable_code)]
extern crate log;

extern crate console_error_panic_hook;

use dlc_messages::{AcceptDlc, Message, OfferDlc, SignDlc};
use wasm_bindgen::prelude::*;
use web_sys::Response;

use core::panic;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
    vec,
};

use js_sys::Uint8Array;

use bitcoin::{Address, Network};
use dlc_manager::{manager::Manager, Oracle, SystemTimeProvider};
// use dlc_sled_storage_provider::SledStorageProvider;
// use electrs_blockchain_provectrsBlockchainProvider;

use log::{debug, info, warn};
// use mock_blockchain_provider::MockBlockchainProvider;
use mocks::{memory_storage_provider::MemoryStorage, mock_blockchain::MockBlockchain};

use simple_wallet::SimpleWallet;

// use crate::storage::storage_provider::StorageProvider;
use oracle_client::P2PDOracleClient;
use serde::{Deserialize, Serialize};

mod oracle_client;
// mod storage;
mod utils;
#[macro_use]
mod macros;

type DlcManager = Manager<
    Arc<SimpleWallet<Arc<MockBlockchain>, Arc<MemoryStorage>>>,
    Arc<MockBlockchain>,
    Box<MemoryStorage>,
    Arc<P2PDOracleClient>,
    Arc<SystemTimeProvider>,
    Arc<MockBlockchain>,
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

#[wasm_bindgen]
pub struct JsDLCInterface {
    options: JsDLCInterfaceOptions,
    manager: Arc<Mutex<DlcManager>>,
}

// #[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsDLCInterfaceOptions {
    oracle_url: String,
    network: String,
    electrs_host: String,
}

impl Default for JsDLCInterfaceOptions {
    fn default() -> Self {
        Self {
            oracle_url: "https://testnet.dlc.link/oracle".to_string(),
            network: "regtest".to_string(),
            electrs_host: "https://blockstream.info/testnet/api/".to_string(),
        }
    }
}

#[wasm_bindgen]
impl JsDLCInterface {
    pub async fn new() -> JsDLCInterface {
        console_error_panic_hook::set_once();
        let options = JsDLCInterfaceOptions::default();
        let active_network: Network = options
            .network
            .parse::<Network>()
            .expect("Must use a valid bitcoin network");

        // ELECTRUM / ELECTRS
        let blockchain = Arc::new(MockBlockchain {});

        // Set up DLC store
        let store = MemoryStorage::new();

        // Set up wallet store
        // I think this needs to change b/c the simple wallet won't work in wasm?
        let root_sled_path: String = "wallet_db".to_string();

        let sled_path = format!("{root_sled_path}_{}", active_network);
        let wallet_store = Arc::new(MemoryStorage::new());

        // Set up wallet
        let wallet = Arc::new(SimpleWallet::new(
            blockchain.clone(),
            wallet_store.clone(),
            active_network,
        ));

        clog!("options: {:?}", options);

        // Set up Oracle Client
        let p2p_client: P2PDOracleClient = P2PDOracleClient::new(&options.oracle_url)
            .await
            .expect("To be able to connect to the oracle");

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

        JsDLCInterface { options, manager }
    }

    pub async fn receive_offer(&self, dlc_offer_message: Vec<u8>) -> Uint8Array {
        let dlc_offer_message: OfferDlc = serde_json::from_slice(&dlc_offer_message).unwrap();
        match self.manager.lock().unwrap().on_dlc_message(
            &Message::Offer(dlc_offer_message.clone()),
            STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
        ) {
            Ok(_) => (),
            Err(e) => {
                info!("DLC manager - receive offer error: {}", e.to_string());
                panic!();
            }
        }

        let temporary_contract_id = dlc_offer_message.temporary_contract_id;

        let (_contract_id, _public_key, accept_msg) = self
            .manager
            .lock()
            .unwrap()
            .accept_contract_offer(&temporary_contract_id)
            .expect("Error accepting contract offer");

        let accept_msg = serde_json::to_vec(&accept_msg).unwrap();
        // serde_wasm_bindgen::to_value(&accept_msg).unwrap()
        js_sys::Uint8Array::from(&accept_msg[..])
        // return accept_msg;
    }

    pub async fn sign_offer(&self, dlc_sign_message: Vec<u8>) -> () {
        let dlc_sign_message: SignDlc = serde_json::from_slice(&dlc_sign_message).unwrap();
        match self.manager.lock().unwrap().on_dlc_message(
            &Message::Sign(dlc_sign_message),
            STATIC_COUNTERPARTY_NODE_ID.parse().unwrap(),
        ) {
            Ok(_) => (),
            Err(e) => {
                info!("DLC manager - sign offer error: {}", e.to_string());
                panic!()
            }
        }
    }
}

// Can remove this when we implement BDK, assuming BDK also doesn't do reserving (locking) of utxos
fn unlock_utxos(
    wallet: Arc<SimpleWallet<Arc<MockBlockchain>, Arc<MemoryStorage>>>,
    response: Response,
) -> Response {
    info!("Unlocking UTXOs");
    wallet.unreserve_all_utxos();
    return response;
}

fn empty_to_address(
    address: String,
    wallet: Arc<SimpleWallet<Arc<MockBlockchain>, Arc<MemoryStorage>>>,
    response: Response,
) -> Response {
    info!("Unlocking UTXOs");
    match wallet.empty_to_address(&Address::from_str(&address).unwrap()) {
        Ok(_) => info!("Emptied bitcoin to {address}"),
        Err(_) => warn!("Failed emptying bitcoin to {address}"),
    }
    return response;
}
