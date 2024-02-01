#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]

extern crate core;
extern crate log;

use bdk::database::MemoryDatabase;
use bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use bdk::wallet::AddressIndex;
use bdk::{descriptor, KeychainKind, SignOptions, Wallet};
use bitcoin::consensus::deserialize;
use bitcoin::hashes::hex::{FromHex, ToHex};
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
use bitcoin::{Network, PrivateKey};
use serde_json::json;
use wasm_bindgen::prelude::*;

use lightning::util::ser::{Readable, Writeable};

use secp256k1_zkp::rand::thread_rng;
use secp256k1_zkp::{
    hashes::*, All, KeyPair, Message, Secp256k1, SecretKey, XOnlyPublicKey as SchnorrPublicKey,
};
use std::io::Cursor;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod oracle;
use oracle::Oracle;

use oracle::error::GenericOracleError;
use oracle::{DbValue, PsbtDbValue};

use dlc_messages::oracle_msgs::{
    DigitDecompositionEventDescriptor, EventDescriptor, OracleAnnouncement, OracleAttestation,
    OracleEvent,
};

mod error;
use error::AttestorError;

extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! clog {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

#[wasm_bindgen]
pub struct Attestor {
    oracle: Oracle,
    secret_key: SecretKey,
}

#[wasm_bindgen]
impl Attestor {
    pub async fn new(
        storage_api_endpoint: String,
        x_secret_key_str: String,
    ) -> Result<Attestor, JsValue> {
        clog!(
            "[WASM-ATTESTOR]: Creating new attestor with storage_api_endpoint: {}",
            storage_api_endpoint
        );
        let secp = Secp256k1::new();
        let xpriv_key = ExtendedPrivKey::from_str(&x_secret_key_str)
            .map_err(|_| JsValue::from_str("Unable to decode xpriv env variable"))?;
        let external_derivation_path = DerivationPath::from_str("m/44h/0h/0h/0")
            .map_err(|_| JsValue::from_str("A valid derivation path"))?;
        let derived_ext_xpriv = xpriv_key
            .derive_priv(
                &secp,
                &external_derivation_path.extend([
                    ChildNumber::Normal { index: 0 },
                    ChildNumber::Normal { index: 0 },
                ]),
            )
            .map_err(|_| {
                JsValue::from_str(
                    "Should be able to derive the private key path during wallet setup",
                )
            })?;
        let secret_key = derived_ext_xpriv.private_key;
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let oracle = Oracle::new(key_pair, secp, storage_api_endpoint)
            .map_err(|_| JsValue::from_str("Error creating Oracle"))?;
        Ok(Attestor { oracle, secret_key })
    }

    pub async fn get_health() -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&json!({"data": [
            {"status": "healthy", "message": ""}
        ]}))?)
    }

    pub async fn create_event(
        &self,
        uuid: &str,
        maturation: &str,
        chain: &str,
    ) -> Result<(), JsValue> {
        let maturation = OffsetDateTime::parse(maturation, &Rfc3339)
            .map_err(|_| JsValue::from_str("Unable to parse maturation time"))?;

        clog!(
            "[WASM-ATTESTOR] Creating event for uuid: {} and maturation_time : {} on chain: {}",
            uuid,
            maturation,
            chain
        );

        let (announcement_obj, outstanding_sk_nonces) = build_announcement(
            &self.oracle.key_pair,
            &self.oracle.secp,
            maturation,
            uuid.to_string(),
        )
        .map_err(|_| JsValue::from_str("Error building announcement"))?;

        let db_value = DbValue(
            Some(outstanding_sk_nonces),
            announcement_obj.encode(),
            None,
            None,
            uuid.to_string(),
            Some(chain.to_string()),
        );

        let new_event = serde_json::to_string(&db_value)
            .map_err(|_| JsValue::from_str("Error serializing new_event to JSON"))?
            .into_bytes();

        match &self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .insert(uuid.to_string(), new_event.clone(), self.secret_key)
            .await
        {
            Ok(Some(_val)) => Ok(()),
            _ => {
                clog!(
                    "[WASM-ATTESTOR] Event was unable to update in StorageAPI with uuid: {}, failed to create event",
                    uuid
                );
                Err(JsValue::from_str("Failed to create event"))
            }
        }
    }

    pub async fn attest(&self, uuid: String, outcome: u64) -> Result<(), JsError> {
        clog!("[WASM-ATTESTOR] retrieving oracle event with uuid {}", uuid);
        let mut event: DbValue;

        let res = match self
            .oracle
            .event_handler
            .storage_api
            .get(uuid.clone(), self.secret_key)
            .await
        {
            Ok(val) => val,
            Err(e) => {
                let message = format!(
                    "[WASM-ATTESTOR] Error retrieving event from StorageAPI: {:?}",
                    e
                );
                clog!("{}", message);
                return Err(JsError::new(&message));
            }
        };
        let event_vec = match res {
            Some(val) => val,
            None => {
                let error_message = format!(
                    "[WASM-ATTESTOR] Event missing in StorageAPI with uuid: {}",
                    uuid
                );
                clog!("{}", error_message);
                return Err(JsError::new(&error_message));
            }
        };
        event = serde_json::from_str(&String::from_utf8_lossy(&event_vec)).map_err(|e| {
            let message = format!(
                "[WASM-ATTESTOR] Error deserializing event from StorageAPI: {:?}",
                e
            );
            clog!("{}", message);
            JsError::new(&message)
        })?;

        let outstanding_sk_nonces = match event.clone().0 {
            Some(value) => value,
            None => return Err(JsError::new("Error: event is None")),
        };

        let announcement = OracleAnnouncement::read(&mut Cursor::new(&event.1)).map_err(|e| {
            let message = format!(
                "[WASM-ATTESTOR] Error reading announcement from StorageAPI: {:?}",
                e
            );
            clog!("{}", message);
            JsError::new(&message)
        })?;

        let num_digits_to_sign = match announcement.oracle_event.event_descriptor {
            dlc_messages::oracle_msgs::EventDescriptor::DigitDecompositionEvent(e) => e.nb_digits,
            _ => {
                return Err(AttestorError::OracleEventNotFoundError(
                    "Got an unexpected EventDescriptor type!".to_string(),
                )
                .into())
            }
        };

        // Here, we take the outcome of the DLC (0-10000), break it down into binary, break it into a vec of characters
        let outcomes = format!("{:0width$b}", outcome, width = num_digits_to_sign as usize)
            .chars()
            .map(|char| char.to_string())
            .collect::<Vec<_>>();

        let attestation = build_attestation(
            outstanding_sk_nonces,
            self.oracle.get_keypair(),
            self.oracle.get_secp(),
            outcomes,
        );

        event.3 = Some(outcome);
        event.2 = Some(attestation.encode());

        let new_event = serde_json::to_string(&event)
            .map_err(|_| JsError::new("[WASM-ATTESTOR] Error serializing new_event to JSON"))?
            .into_bytes();

        let res = match self
            .oracle
            .event_handler
            .storage_api
            .insert(uuid.clone(), new_event.clone(), self.secret_key)
            .await
        {
            Ok(val) => val,
            Err(e) => {
                clog!(
                    "[WASM-ATTESTOR] Error updating event in StorageAPI: {:?}",
                    e
                );
                None
            }
        };
        let _insert_event = match res {
            Some(val) => Some(val),
            None => {
                clog!(
                    "[WASM-ATTESTOR] Event was unable to update in StorageAPI with uuid: {}",
                    uuid
                );
                None
            }
        };
        Ok(())
    }

    pub async fn get_events(&self) -> Result<JsValue, JsValue> {
        let events = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get_all(self.secret_key)
            .await
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error getting all events"))?;

        let events = match events {
            Some(value) => value,
            None => return Err(JsValue::from_str("[WASM-ATTESTOR] Error: events is None")),
        };

        let events: Result<Vec<ApiOracleEvent>, JsValue> = events
            .iter()
            .map(|event| parse_database_entry(event.clone().1))
            .collect();

        let events = events?;

        serde_wasm_bindgen::to_value(&events)
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error serializing events to JSON"))
    }

    pub async fn get_event(&self, uuid: String) -> Result<JsValue, JsValue> {
        let result = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get(uuid, self.secret_key)
            .await
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error getting event"))?;

        match result {
            Some(event) => {
                let parsed_event = parse_database_entry(event).map_err(|_| {
                    JsValue::from_str("[WASM-ATTESTOR] Error parsing database entry")
                })?;
                serde_wasm_bindgen::to_value(&parsed_event).map_err(|_| {
                    JsValue::from_str("[WASM-ATTESTOR] Error serializing event to JSON")
                })
            }
            None => Ok(JsValue::NULL),
        }
    }

    pub async fn get_pubkey(&self) -> String {
        SchnorrPublicKey::from_keypair(&self.oracle.key_pair)
            .0
            .to_string()
    }

    pub async fn create_psbt_event(
        &self,
        uuid: &str, // can use the txid of the prefunding tx here
        // funding_psbt: &str,
        closing_psbt: &str,
        mint_address: &str,
        chain: &str,
    ) -> Result<(), JsValue> {
        clog!(
            "[WASM-ATTESTOR] Creating new psbt event with uuid: {}",
            uuid
        );
        // let funding_psbt: Vec<u8> = FromHex::from_hex(funding_psbt)
        //     .map_err(|_| JsValue::from_str("Error decoding funding_psbt hex"))?;
        // // let funding_psbt: PartiallySignedTransaction = deserialize(&funding_psbt)
        // //     .map_err(|_| JsValue::from_str("Error decoding funding_psbt"))?;

        let closing_psbt: Vec<u8> = FromHex::from_hex(closing_psbt)
            .map_err(|_| JsValue::from_str("Error decoding closing_psbt hex"))?;
        let closing_psbt: PartiallySignedTransaction = deserialize(&closing_psbt[..])
            .map_err(|_| JsValue::from_str("Error decoding closing_psbt"))?;

        // recreate the transactions, and make sure they match, minus the signatures

        // make sure the internal tap key is invalid.

        // here's how we can put the hash-preimage into the witness to spend the htlc
        // https://github.com/lightningdevkit/rust-lightning/blob/main/lightning/src/ln/chan_utils.rs#L712
        // https://github.com/rust-bitcoin/rust-bitcoin/blob/master/bitcoin/examples/taproot-psbt.rs
        // https://github.com/rust-bitcoin/rust-bitcoin/blob/master/bitcoin/examples/taproot-psbt.rs#L686

        // Use the bdk verify functions to verify the sigs on the PSBTs

        // Grab the details for the prefunding tx from the PSBTs somehow.
        // verify that the PSBTs are "good", that the funding tx is good, etc. This should be done via consensus somehow...

        // I guess we could leverage our FROST system for that consensus, by sending a message to the coordinator
        // to verify all the things, and they'll only sign a message about validating the system
        // if the threshold agrees.

        // For now, assuming that's all good...
        // We should store the PSBTs in the database, and then we can use them to sign the funding tx and payout tx later.

        clog!(
            "[WASM-ATTESTOR] the inputs of the closing PSBT is the funding tx: {:?}",
            closing_psbt.inputs
        );

        let psbt_db_value = PsbtDbValue(
            // bitcoin::consensus::encode::serialize(&funding_psbt).to_hex(),
            bitcoin::consensus::encode::serialize(&closing_psbt).to_hex(),
            mint_address.to_string(),
            uuid.to_string(),
            None,                    //outcome
            Some(chain.to_string()), //chain name
        );
        clog!("[WASM-ATTESTOR] psbt_db_value created",);

        let new_psbt_event = serde_json::to_string(&psbt_db_value)
            .map_err(|_| JsValue::from_str("Error serializing new_event to JSON"))?
            .into_bytes();
        clog!("[WASM-ATTESTOR] psbt_db_value serialized",);

        match &self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .insert(uuid.to_string(), new_psbt_event.clone(), self.secret_key)
            .await
        {
            Ok(Some(_val)) => {
                clog!(
                    "[WASM-ATTESTOR] PSBT Event was created in StorageAPI with uuid: {}",
                    uuid
                );
                Ok(())
            }
            _ => {
                clog!(
                    "[WASM-ATTESTOR] Event was unable to update in StorageAPI with uuid: {}, failed to create psbt locking event",
                    uuid
                );
                Err(JsValue::from_str("Failed to create psbt locking event"))
            }
        }
    }

    pub async fn close_psbt_event(&self, uuid: &str) -> Result<JsValue, JsValue> {
        // pull these from the db
        clog!("[WASM-ATTESTOR] Closing psbt event with uuid: {}", uuid);

        let psbt_event = match self.get_psbt_event(uuid.to_string()).await {
            Ok(psbt_event) => psbt_event,
            Err(e) => {
                clog!(
                    "[WASM-ATTESTOR] Error getting psbt event with uuid: {}, error: {:?}",
                    uuid,
                    e
                );
                return Err(JsValue::from_str("Error getting psbt event"));
            }
        };

        clog!("[WASM-ATTESTOR] Closing psbt event with uuid: {}", uuid);

        let mut closing_psbt: PartiallySignedTransaction = psbt_event.closing_psbt;

        // Starting the closing flow of signing and broadcasting the closing transaction
        let secp = Secp256k1::new();
        let xprv = ExtendedPrivKey::from_str("tprv8ZgxMBicQKsPdojQCxUVZorqp1eSvoWwMsnn4PEXwQ8i1KP9dqNffJLFP2kgjgW8petjnkVS5TjLPkruEDasiNiaBt5iG5QQ2MFdioJ9eqL").expect("A valid xprv");

        // Generating derived keys and first address
        let external_derivation_path =
            DerivationPath::from_str("m/86'/1'/2'/1/0").expect("A valid derivation path");

        let signing_external_descriptor = descriptor!(tr((
            xprv,
            external_derivation_path.clone() //.extend([ChildNumber::Normal { index: 0 }])
        )))
        .unwrap();

        let derived_ext_xpriv = match xprv.derive_priv(
            &secp,
            &external_derivation_path.extend([
                ChildNumber::Normal { index: 0 },
                // ChildNumber::Normal { index: 0 },
            ]),
        ) {
            Ok(derived_ext_xpriv) => derived_ext_xpriv,
            Err(e) => {
                clog!(
                    "[WASM-ATTESTOR] Error deriving external key for closing psbt, error: {:?}",
                    e
                );
                return Err(JsValue::from_str(
                    "Error deriving external key for closing psbt",
                ));
            }
        };
        let keypair = KeyPair::from_secret_key(&secp, &derived_ext_xpriv.private_key);
        let pubkey = SchnorrPublicKey::from_keypair(&keypair).0;
        let secret_key = keypair.secret_key();

        let signing_external_descriptor_str = signing_external_descriptor
            .0
            // .at_derivation_index(0)
            .to_string();

        clog!("[WASM-ATTESTOR] building wallet");
        let mut wallet = match Wallet::new(
            signing_external_descriptor,
            None,
            Network::Regtest,
            MemoryDatabase::default(),
        ) {
            Ok(wallet) => wallet,
            Err(e) => {
                clog!(
                    "[WASM-ATTESTOR] Error building wallet for closing psbt, error: {:?}",
                    e
                );
                return Err(JsValue::from_str("Error building wallet for closing psbt"));
            }
        };

        clog!(
            "[WASM-ATTESTOR] {}",
            json!({"xprv": xprv.to_string(), "secret_key": secret_key, "schnorr_public_key": pubkey, "network": Network::Regtest,
                // "signing_internal_descriptor": signing_internal_descriptor.0.at_derivation_index(0).to_string(),
                "signing_external_descriptor": signing_external_descriptor_str
            })
        );

        let send_to = wallet
            .get_address(AddressIndex::New)
            .expect("A valid address");
        clog!(
            "a pubkey to use from the wallet {}, {}, {:?}, is tap {}",
            send_to.script_pubkey(),
            send_to.index,
            send_to.address.address_type(),
            send_to.script_pubkey().is_v1_p2tr() // send_to.script_pubkey().
        );

        let sign_options = SignOptions {
            sign_with_tap_internal_key: false,
            remove_partial_sigs: false,
            try_finalize: false,
            ..Default::default()
        };

        let priv_key = PrivateKey::new(secret_key, Network::Regtest);
        clog!(
            "[WASM-ATTESTOR] signing priv_key: {:?} and pubkey: {}",
            priv_key.inner.display_secret().to_string(),
            priv_key.public_key(&secp)
        );
        let signer = SignerWrapper::new(
            priv_key,
            SignerContext::Tap {
                is_internal_key: false,
            },
        );
        // clog!(
        //     "[WASM-ATTESTOR]  tap scripts control block \n{:?}\nother thing\n{:?}",
        //     closing_psbt.inputs[0]
        //         .tap_scripts
        //         .first_key_value()
        //         .unwrap()
        //         .0,
        //     closing_psbt.inputs[0]
        //         .tap_scripts
        //         .first_key_value()
        //         .unwrap()
        //         .1
        // );
        wallet.add_signer(KeychainKind::External, SignerOrdering(0), Arc::new(signer));

        match wallet.sign(&mut closing_psbt, sign_options) {
            Ok(_) => clog!("[WASM-ATTESTOR] closing_psbt signed"),
            Err(e) => clog!("[WASM-ATTESTOR] closing_psbt not signed, error: {:?}", e),
        };
        // clog!(
        //     "[WASM-ATTESTOR]  tap scripts control block \n{:?}\nother thing\n{:?}",
        //     closing_psbt.inputs[0]
        //         .tap_scripts
        //         .first_key_value()
        //         .unwrap()
        //         .0,
        //     closing_psbt.inputs[0]
        //         .tap_scripts
        //         .first_key_value()
        //         .unwrap()
        //         .1
        // );
        // clog!(
        //     "[WASM-ATTESTOR] closing_psbt after signing: \n{:?}",
        //     closing_psbt.clone()
        // );

        match bdk::miniscript::psbt::PsbtExt::finalize_mut(&mut closing_psbt, &secp) {
            Ok(_) => clog!("[WASM-ATTESTOR] closing_psbt finalized"),
            Err(e) => clog!("[WASM-ATTESTOR] closing_psbt not finalized, error: {:?}", e),
        };

        let closing_tx = closing_psbt.extract_tx();
        println!("closing_tx: {:?}", closing_tx);

        let closing_tx_value = serde_wasm_bindgen::to_value(&closing_tx)
            .map_err(|_| JsValue::from_str("Error serializing closing_tx to JSON"))?;
        Ok(closing_tx_value)
    }

    async fn get_psbt_event(&self, uuid: String) -> Result<ApiOraclePsbtEvent, GenericOracleError> {
        let result = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get(uuid, self.secret_key)
            .await
            .map_err(|_| GenericOracleError {
                message: "Error getting event from storage_api".to_string(),
            })?;

        match result {
            Some(event) => {
                let parsed_event = parse_psbt_database_entry(event)?;
                Ok(parsed_event)
            }
            None => Err(GenericOracleError {
                message: "No event found".to_string(),
            }),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
struct ApiOraclePsbtEvent {
    event_id: String,
    uuid: String,
    // funding_psbt: PartiallySignedTransaction,
    closing_psbt: PartiallySignedTransaction,
    mint_address: String,
    outcome: Option<u64>,
    chain: Option<String>,
}

fn parse_psbt_database_entry(
    event_binary: Vec<u8>,
) -> Result<ApiOraclePsbtEvent, GenericOracleError> {
    // let event_str = String::from_utf8_lossy(&event_binary);
    // let event: PsbtDbValue = serde_json::from_str(&event_str).map_err(|_| GenericOracleError {
    //     message: "Unable to deserialize psbt event from db".to_string(),
    // })?;

    let event: PsbtDbValue =
        serde_json::from_str(&String::from_utf8(event_binary.clone()).expect("to string"))
            .map_err(|_| GenericOracleError {
                message: "Error deserializing new_event from JSON".to_string(),
            })?;

    // let (funding_psbt, closing_psbt) = (event.0.clone(), event.1.clone());
    let closing_psbt = event.0.clone();

    // let funding_psbt: Vec<u8> =
    //     FromHex::from_hex(&funding_psbt).expect("to decode funding_psbt hex");
    // let funding_psbt: PartiallySignedTransaction =
    //     deserialize(&funding_psbt).map_err(|_| GenericOracleError {
    //         message: "Unable to deserialize funding psbt from db value".to_string(),
    //     })?;

    let closing_psbt: Vec<u8> =
        FromHex::from_hex(&closing_psbt).expect("to decode funding_psbt hex");
    let closing_psbt: PartiallySignedTransaction =
        deserialize(&closing_psbt).map_err(|_| GenericOracleError {
            message: "Unable to deserialize closing psbt event from db".to_string(),
        })?;

    Ok(ApiOraclePsbtEvent {
        event_id: event.2.clone(),
        uuid: event.2,
        // funding_psbt,
        closing_psbt,
        mint_address: event.1,
        outcome: event.3,
        chain: event.4,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SortOrder {
    Insertion,
    ReverseInsertion,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Filters {
    sort_by: SortOrder,
    page: u32,
    // asset_pair: AssetPair,
    maturation: String,
    outcome: Option<u64>,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            sort_by: SortOrder::ReverseInsertion,
            page: 0,
            // asset_pair: AssetPair::BTCUSD,
            maturation: "".to_string(),
            outcome: None,
        }
    }
}

#[derive(Serialize, Debug)]
struct ApiOracleEvent {
    event_id: String,
    uuid: String,
    rust_announcement_json: String,
    rust_announcement: String,
    rust_attestation_json: Option<String>,
    rust_attestation: Option<String>,
    maturation: String,
    outcome: Option<u64>,
    chain: Option<String>,
}

fn parse_database_entry(event: Vec<u8>) -> Result<ApiOracleEvent, JsValue> {
    let event_str = String::from_utf8_lossy(&event);
    let event: DbValue = serde_json::from_str(&event_str)
        .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error parsing event from string"))?;

    let announcement_vec = event.1.clone();
    let mut cursor = Cursor::new(&announcement_vec);
    let announcement = OracleAnnouncement::read(&mut cursor)
        .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error reading OracleAnnouncement"))?;

    let db_att = event.2.clone();
    let decoded_att_json = match db_att {
        None => None,
        Some(att_vec) => {
            let mut attestation_cursor = Cursor::new(&att_vec);

            match OracleAttestation::read(&mut attestation_cursor) {
                Ok(att) => Some(format!("{:?}", att)),
                Err(_) => Some("[WASM-ATTESTOR] Error decoding attestation".to_string()),
            }
        }
    };

    let rust_announcement_json = serde_json::to_string(&announcement)
        .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error serializing announcement to JSON"))?;

    Ok(ApiOracleEvent {
        event_id: announcement.oracle_event.event_id.clone(),
        uuid: event.4,
        rust_announcement_json,
        rust_announcement: event.1.to_hex(),
        rust_attestation_json: decoded_att_json,
        rust_attestation: event.2.map(|att| att.to_hex()),
        maturation: announcement.oracle_event.event_maturity_epoch.to_string(),
        outcome: event.3,
        chain: event.5,
    })
}

pub fn generate_nonces_for_event(
    secp: &Secp256k1<All>,
    event_descriptor: &EventDescriptor,
) -> (Vec<SchnorrPublicKey>, Vec<SecretKey>) {
    let nb_nonces = match event_descriptor {
        EventDescriptor::DigitDecompositionEvent(d) => d.nb_digits,
        EventDescriptor::EnumEvent(_) => panic!(),
    };

    let priv_nonces: Vec<_> = (0..nb_nonces)
        .map(|_| SecretKey::new(&mut thread_rng()))
        .collect();
    let key_pairs: Vec<_> = priv_nonces
        .iter()
        .map(|x| {
            KeyPair::from_seckey_slice(secp, x.as_ref())
                .expect("[WASM-ATTESTOR] Failed to generate keypair from secret key")
        })
        .collect();

    let nonces = key_pairs
        .iter()
        .map(|k| SchnorrPublicKey::from_keypair(k).0)
        .collect();

    (nonces, priv_nonces)
}

pub fn build_announcement(
    keypair: &KeyPair,
    secp: &Secp256k1<All>,
    maturation: OffsetDateTime,
    event_id: String,
) -> Result<(OracleAnnouncement, Vec<SecretKey>), secp256k1_zkp::UpstreamError> {
    let event_descriptor =
        EventDescriptor::DigitDecompositionEvent(DigitDecompositionEventDescriptor {
            base: 2,
            is_signed: false,
            unit: "BTCUSD".to_string(),
            precision: 0,
            nb_digits: 14u16,
        });
    let (oracle_nonces, sk_nonces) = generate_nonces_for_event(secp, &event_descriptor);
    let oracle_event = OracleEvent {
        oracle_nonces,
        event_maturity_epoch: maturation
            .unix_timestamp()
            .try_into()
            .expect("[WASM-ATTESTOR] Failed to convert maturation to event_maturity_epoch"),
        event_descriptor: event_descriptor.clone(),
        event_id: event_id.to_string(),
    };
    let mut event_hex = Vec::new();
    oracle_event
        .write(&mut event_hex)
        .expect("[WASM-ATTESTOR] Error writing oracle event");
    let msg = Message::from_hashed_data::<secp256k1_zkp::hashes::sha256::Hash>(&event_hex);
    let sig = secp.sign_schnorr(&msg, keypair);
    let announcement = OracleAnnouncement {
        oracle_event,
        oracle_public_key: keypair.public_key().into(),
        announcement_signature: sig,
    };
    Ok((announcement, sk_nonces))
}

pub fn build_attestation(
    outstanding_sk_nonces: Vec<SecretKey>,
    key_pair: &KeyPair,
    secp: &Secp256k1<All>,
    outcomes: Vec<String>,
) -> OracleAttestation {
    let nonces = outstanding_sk_nonces;
    let signatures = outcomes
        .iter()
        .zip(nonces.iter())
        .map(|(x, nonce)| {
            let msg =
                Message::from_hashed_data::<secp256k1_zkp::hashes::sha256::Hash>(x.as_bytes());
            dlc::secp_utils::schnorrsig_sign_with_nonce(secp, &msg, key_pair, nonce.as_ref())
        })
        .collect();
    OracleAttestation {
        oracle_public_key: key_pair.public_key().into(),
        signatures,
        outcomes,
    }
}
