#![feature(async_fn_in_trait)]
#![allow(unreachable_code)]
#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]

extern crate console_error_panic_hook;
extern crate log;

use bitcoin::{Network, PrivateKey};
use dlc_messages::{Message, OfferDlc, SignDlc};
use secp256k1_zkp::UpstreamError;
use wasm_bindgen::prelude::*;

use lightning::util::ser::Readable;

use secp256k1_zkp::hashes::*;
use secp256k1_zkp::Secp256k1;

use core::panic;
use std::fmt;
use std::{io::Cursor, str::FromStr, sync::Arc};

use dlc_manager::{contract::Contract, ContractId, SystemTimeProvider};

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
    manager: DlcManager,
    wallet: Arc<JSInterfaceWallet>,
    blockchain: Arc<EsploraAsyncBlockchainProviderJsWallet>,
}

// #[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsDLCInterfaceOptions {
    network: String,
    electrs_url: String,
    address: String,
}

impl Default for JsDLCInterfaceOptions {
    // Default values for Manager Options
    fn default() -> Self {
        Self {
            network: "regtest".to_string(),
            electrs_url: "https://devnet.dlc.link/electrs".to_string(),
            address: "".to_string(),
        }
    }
}

fn to_wallet_error<T>(e: T) -> WalletError
where
    T: std::fmt::Display,
{
    WalletError(e.to_string())
}

#[wasm_bindgen]
impl JsDLCInterface {
    pub async fn new(
        privkey: String,
        address: String,
        network: String,
        electrs_url: String,
        storage_api_url: String,
    ) -> Result<JsDLCInterface, JsError> {
        console_error_panic_hook::set_once();

        let options = JsDLCInterfaceOptions {
            network,
            electrs_url,
            address,
        };

        let active_network: Network = options.network.parse::<Network>()?;

        let blockchain: Arc<EsploraAsyncBlockchainProviderJsWallet> =
            Arc::new(EsploraAsyncBlockchainProviderJsWallet::new(
                options.electrs_url.to_string(),
                active_network,
            ));

        // Generate keypair from secret key
        let seckey = secp256k1_zkp::SecretKey::from_str(&privkey)
            .map_err(|e| JsError::new(&format!("Error parsing private key: {}", e)))?;

        let secp = Secp256k1::new();

        // let pubkey = PublicKey::from_secret_key(&secp, &seckey);
        let pubkey =
            bitcoin::PublicKey::from_private_key(&secp, &PrivateKey::new(seckey, active_network));

        // Set up DLC store
        let dlc_store = AsyncStorageApiProvider::new(pubkey.to_string(), seckey, storage_api_url);

        // Set up wallet
        let wallet = Arc::new(JSInterfaceWallet::new(
            options.address.to_string(),
            PrivateKey::new(seckey, active_network),
        ));

        // Set up time provider
        let time_provider = SystemTimeProvider {};

        // Create the DLC Manager
        let manager = Manager::new(
            Arc::clone(&wallet),
            Arc::clone(&blockchain),
            Box::new(dlc_store),
            None,
            Arc::new(time_provider),
        )?;

        Ok(JsDLCInterface {
            options,
            manager,
            wallet,
            blockchain,
        })
    }

    pub fn get_options(&self) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(&self.options)?)
    }

    pub async fn get_wallet_balance(&self) -> Result<u64, JsError> {
        self.blockchain
            .refresh_chain_data(self.options.address.clone())
            .await
            .map_err(|_e| {
                JsError::new(
                    "Failed to communicate with the Bitcoin blockchain. Please try again later!",
                )
            })?;

        self.wallet.set_utxos(
            self.blockchain
                .get_utxos()
                .map_err(|_e| JsError::new("Failed to set UTXOs. Please try again later!"))?,
        )?;

        self.blockchain.get_balance().await.map_err(|_e| {
            JsError::new("Failed to retrieve the Bitcoin balance. Please try again later!")
        })
    }

    // public async function for fetching all the contracts on the manager
    pub async fn get_contracts(&self) -> Result<JsValue, JsError> {
        log_to_console!("get_contracts");
        // let mut errors: Vec<WalletError> = vec![];
        let contracts: Vec<JsContract> = self
            .manager
            .get_store()
            .get_contracts()
            .await?
            .into_iter()
            .map(|c| match JsContract::from_contract(c.clone()) {
                Ok(c) => Ok(c),
                Err(e) => {
                    log_to_console!("Error getting contract with id {:?}: {}", c.get_id(), e);
                    Err(e)
                }
            })
            .filter_map(Result::ok)
            .collect();

        Ok(serde_wasm_bindgen::to_value(&contracts)?)
    }

    // public async function for fetching one contract as a JsContract type
    pub async fn get_contract(&self, contract_str: String) -> Result<JsValue, JsError> {
        let contract_id =
            ContractId::read(&mut Cursor::new(&contract_str)).map_err(to_wallet_error)?;
        let contract = self.manager.get_store().get_contract(&contract_id).await?;
        match contract {
            Some(contract) => Ok(serde_wasm_bindgen::to_value(&JsContract::from_contract(
                contract,
            )?)?),
            None => Ok(JsValue::NULL),
        }
    }

    pub async fn accept_offer(&self, offer_json: String) -> Result<String, JsError> {
        //could consider doing a refresh_chain_data here to have the newest utxos

        let accept_msg_result = async {
            let dlc_offer_message: OfferDlc =
                serde_json::from_str(&offer_json).map_err(to_wallet_error)?;
            let temporary_contract_id = dlc_offer_message.temporary_contract_id;

            let counterparty = STATIC_COUNTERPARTY_NODE_ID
                .parse()
                .map_err(|e: UpstreamError| WalletError(e.to_string()))?;
            self.manager
                .on_dlc_message(&Message::Offer(dlc_offer_message.clone()), counterparty)
                .await
                .map_err(to_wallet_error)?;
            let (_contract_id, _public_key, accept_msg) = self
                .manager
                .accept_contract_offer(&temporary_contract_id)
                .await
                .map_err(to_wallet_error)?;
            serde_json::to_string(&accept_msg).map_err(to_wallet_error)
        };
        match accept_msg_result.await {
            Ok(accept_msg) => Ok(accept_msg),
            Err(e) => {
                log_to_console!("Error accepting offer: {}", e);
                Err(JsError::new(&format!("Error accepting offer: {}", e)))
            }
        }
    }

    pub async fn countersign_and_broadcast(
        &self,
        dlc_sign_message: String,
    ) -> Result<String, JsError> {
        let dlc_sign_result = async {
            let dlc_sign_message: SignDlc =
                serde_json::from_str(&dlc_sign_message).map_err(to_wallet_error)?;
            self.manager
                .on_dlc_message(
                    &Message::Sign(dlc_sign_message.clone()),
                    STATIC_COUNTERPARTY_NODE_ID
                        .parse()
                        .map_err(to_wallet_error)?,
                )
                .await
                .map_err(to_wallet_error)?;
            let store = self.manager.get_store();
            let contract = store
                .get_signed_contracts()
                .await
                .map_err(to_wallet_error)?
                .into_iter()
                .find(|c| c.accepted_contract.get_contract_id() == dlc_sign_message.contract_id);
            match contract {
                None => Err(WalletError(
                    "DLC Manager: - Sign Offer Error: Contract not found".to_string(),
                )),
                Some(c) => Ok(c.accepted_contract.dlc_transactions.fund.txid().to_string())
                    as Result<String, WalletError>,
            }
        };
        match dlc_sign_result.await {
            Ok(txid) => Ok(txid),
            Err(e) => {
                log_to_console!("Error signing and broadcasting: {}", e);
                Err(JsError::new(&format!(
                    "Error signing and broadcasting: {}",
                    e
                )))
            }
        }
    }

    pub async fn reject_offer(&self, contract_id: String) -> Result<(), JsError> {
        let reject_result = async {
            let contract_id =
                ContractId::read(&mut Cursor::new(&contract_id)).map_err(to_wallet_error)?;
            let contract = self
                .manager
                .get_store()
                .get_contract(&contract_id)
                .await
                .map_err(to_wallet_error)?;

            if let Some(Contract::Offered(c)) = contract {
                self.manager
                    .get_store()
                    .update_contract(&Contract::Rejected(c))
                    .await
                    .map_err(to_wallet_error)?;
            }
            Ok(()) as Result<(), WalletError>
        };
        match reject_result.await {
            Ok(_) => Ok(()),
            Err(e) => {
                log_to_console!("Error signing and broadcasting: {}", e);
                Err(JsError::new(&format!(
                    "Error signing and broadcasting: {}",
                    e
                )))
            }
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
    fn from_contract(contract: Contract) -> Result<JsContract, WalletError> {
        let state = match contract {
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

        fn hex_str(value: &[u8]) -> Result<String, std::fmt::Error> {
            let mut res = String::with_capacity(64);
            for v in value {
                write!(res, "{:02x}", v)?;
            }
            Ok(res)
        }

        Ok(JsContract {
            id: hex_str(&contract.get_id()).map_err(to_wallet_error)?,
            state: state.to_string(),
            acceptor_collateral,
            tx_id,
        })
    }
}
