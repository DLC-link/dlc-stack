#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]

extern crate core;
extern crate log;
use ::hex::ToHex;
use bitcoin::consensus::deserialize;
use bitcoin::hashes::hex::FromHex;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
use serde_json::json;
use wasm_bindgen::prelude::*;

use lightning::util::ser::{Readable, Writeable};

use secp256k1_zkp::rand::thread_rng;
use secp256k1_zkp::{
    hashes::*, All, KeyPair, Message, PublicKey, Secp256k1, SecretKey,
    XOnlyPublicKey as SchnorrPublicKey,
};
use std::io::Cursor;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod oracle;
use oracle::{DbValue, Oracle, PsbtDbValue};

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

        let events: Result<Vec<ApiOracleEvent>, JsValue> = events
            .iter()
            .map(|event| parse_database_entry(event.clone().1))
            .collect();
        let events = events?;

        serde_wasm_bindgen::to_value(&events)
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error serializing events to JSON"))
    }

    pub async fn get_psbt_events(&self) -> Result<JsValue, JsValue> {
        let events = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get_all(self.secret_key)
            .await
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error getting all events"))?;

        let events: Result<Vec<ApiOraclePsbtEvent>, JsValue> = events
            .iter()
            .map(|event| parse_psbt_database_entry(event.clone().1))
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

    pub async fn get_psbt_event(&self, uuid: String) -> Result<JsValue, JsValue> {
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
                let parsed_event = parse_psbt_database_entry(event).map_err(|_| {
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
        uuid: &str,
        psbt1: &str,
        psbt2: &str,
        mint_address: &str,
        chain: &str,
    ) -> Result<(), JsValue> {
        let psbt1: Vec<u8> =
            FromHex::from_hex(psbt1).map_err(|_| JsValue::from_str("Error decoding psbt1 hex"))?;
        let psbt2: Vec<u8> =
            FromHex::from_hex(psbt2).map_err(|_| JsValue::from_str("Error decoding psbt2 hex"))?;

        let psbt1: bitcoin::psbt::PartiallySignedTransaction =
            deserialize(&psbt1).map_err(|_| JsValue::from_str("Error decoding psbt1"))?;
        let psbt2: bitcoin::psbt::PartiallySignedTransaction =
            deserialize(&psbt2[..]).map_err(|_| JsValue::from_str("Error decoding psbt2"))?;

        // clog!("psbt1: {:?}", psbt1);
        // clog!("\n");
        // clog!("psbt2: {:?}", psbt2);

        // Grab the details for the prefunding tx from the PSBTs somehow.
        // verify that the PSBTs are good, that the funding tx is good, etc. This should be done via consensus somehow...

        // I guess we could leverage our FROST system for that consensus, by sending a message to the coordinator
        // to verify all the things, and they'll only sign a message about validating the system
        // if the threshold agrees.

        // For now, assuming that's all good...
        // We should store the PSBTs in the database, and then we can use them to sign the funding tx and payout tx later.

        let psbt_db_value = PsbtDbValue(
            bitcoin::consensus::encode::serialize(&psbt1),
            bitcoin::consensus::encode::serialize(&psbt2),
            mint_address.to_string(),
            uuid.to_string(),
            None,                    //outcome
            Some(chain.to_string()), //chain name
        );

        let new_psbt_event = serde_json::to_string(&psbt_db_value)
            .map_err(|_| JsValue::from_str("Error serializing new_event to JSON"))?
            .into_bytes();

        match &self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .insert(uuid.to_string(), new_psbt_event.clone(), self.secret_key)
            .await
        {
            Ok(Some(_val)) => Ok(()),
            _ => {
                clog!(
                    "[WASM-ATTESTOR] Event was unable to update in StorageAPI with uuid: {}, failed to create psbt locking event",
                    uuid
                );
                Err(JsValue::from_str("Failed to create psbt locking event"))
            }
        }
    }
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
struct ApiOraclePsbtEvent {
    event_id: String,
    uuid: String,
    psbt1: String,
    psbt2: String,
    mint_address: String,
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
        rust_announcement: event.1.encode_hex::<String>(),
        rust_attestation_json: decoded_att_json,
        rust_attestation: event.2.map(|att| att.encode_hex::<String>()),
        maturation: announcement.oracle_event.event_maturity_epoch.to_string(),
        outcome: event.3,
        chain: event.5,
    })
}

fn parse_psbt_database_entry(event_binary: Vec<u8>) -> Result<ApiOraclePsbtEvent, JsValue> {
    let event_str = String::from_utf8_lossy(&event_binary);
    let event: PsbtDbValue = serde_json::from_str(&event_str)
        .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error parsing event from string"))?;

    let (psbt1, psbt2) = (event.0.clone(), event.1.clone());

    let psbt1: bitcoin::psbt::PartiallySignedTransaction =
        deserialize(&psbt1).map_err(|_| JsValue::from_str("Error decoding psbt1"))?;
    let psbt2: bitcoin::psbt::PartiallySignedTransaction =
        deserialize(&psbt2).map_err(|_| JsValue::from_str("Error decoding psbt2"))?;

    Ok(ApiOraclePsbtEvent {
        event_id: event.3.clone(),
        uuid: event.3,
        psbt1: serde_json::to_string(&psbt1)
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error serializing psbt1 to JSON"))?,
        psbt2: serde_json::to_string(&psbt2)
            .map_err(|_| JsValue::from_str("[WASM-ATTESTOR] Error serializing psbt2 to JSON"))?,
        mint_address: event.2,
        outcome: event.4,
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

#[cfg(test)]
mod tests {
    use super::*;

    // use crate::blockdata::locktime::PackedLockTime;
    // use crate::hash_types::Txid;
    // use crate::hashes::hex::FromHex;
    // use crate::hashes::{hash160, ripemd160, sha256, Hash};

    // use secp256k1::{self, Secp256k1};

    // use crate::blockdata::script::Script;
    // use crate::blockdata::transaction::{OutPoint, Sequence, Transaction, TxIn, TxOut};
    // use crate::consensus::encode::{deserialize, serialize, serialize_hex};
    // use crate::internal_macros::hex_script;
    // use crate::network::constants::Network::Bitcoin;
    // use crate::util::bip32::{ChildNumber, ExtendedPrivKey, ExtendedPubKey, KeySource};
    // use crate::util::psbt::map::{Input, Output};
    // use crate::util::psbt::raw;

    // use crate::blockdata::witness::Witness;
    use std::collections::BTreeMap;

    // use bitcoin::psbt;
    // use wasm_bindgen::prelude::*;
    // use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::*;

    #[test]
    fn trivial_psbt() {
        let psbt = bitcoin::util::psbt::PartiallySignedTransaction {
            unsigned_tx: bitcoin::Transaction {
                version: 2,
                lock_time: bitcoin::blockdata::locktime::PackedLockTime::ZERO,
                input: vec![],
                output: vec![],
            },
            xpub: Default::default(),
            version: 0,
            proprietary: BTreeMap::new(),
            unknown: BTreeMap::new(),

            inputs: vec![],
            outputs: vec![],
        };
        assert_eq!(
            bitcoin::consensus::encode::serialize_hex(&psbt),
            "70736274ff01000a0200000000000000000000"
        );

        let psbt1: Vec<u8> =
            FromHex::from_hex("70736274ff0100710200000001c6a7269431c21132ad0db2c46ce7a71a79df57d9201f31fe46ee1b3d82f71ccf0000000000ffffffff0250c3000000000000160014622c23eebbf46df254d7da8e1c4d95d4f5c7d69f961cf50500000000160014b500f5f6ce6b3aa7f5a871c49ab08ff38b45150100000000000100df0100000001ce19e37c126c555f00d1af024e95dd7c7681d42831d27388897456bcf38bdff5010000006b483045022100994d614b679eed66fc5b0a2a85dcf0d993a6a8575b8af5fc164ff31015ae4a680220215badf69a74365046504141a1f2b1ce85b7280efd98fcf0a182ab8be999925c012102add319140c528a8955d76d4afe32c4d3143fea57ea353a31ce793cffb77ef861fdffffff0200e1f50500000000160014b500f5f6ce6b3aa7f5a871c49ab08ff38b451501b0fa6459000000001976a9142b19bade75a48768a5ffc142a86490303a95f41388ac00000000000000").expect("to decode psbt1 hex");
        println!("psbt1: {:?}", psbt1);
        assert_eq!(psbt1.len(), 351);

        let psbt1: bitcoin::psbt::PartiallySignedTransaction =
            deserialize(&psbt1).expect("to decode psbt1");
        assert_eq!(psbt1.inputs.len(), 1);
    }

    #[wasm_bindgen_test]
    async fn setup_new_bridge_lock_test() {
        let attestor = Attestor::new("https://devnet.dlc.link/storage-api".to_string(), "xprv9s21ZrQH143K3UL4H8EBxF8trNkq3Pfs4ZhpSKe56oX75rJ1b6Jkj2CAcbYkdb2KASLp5LoubpzqF2KwMHUBtGKPj9DfaiZfLByUTtFkwPu".to_string())
            .await
            .expect("To create a test attestor");

        attestor
            .create_psbt_event(
                "testuuid",
                "70736274ff0100710200000001c6a7269431c21132ad0db2c46ce7a71a79df57d9201f31fe46ee1b3d82f71ccf0000000000ffffffff0250c3000000000000160014622c23eebbf46df254d7da8e1c4d95d4f5c7d69f961cf50500000000160014b500f5f6ce6b3aa7f5a871c49ab08ff38b45150100000000000100df0100000001ce19e37c126c555f00d1af024e95dd7c7681d42831d27388897456bcf38bdff5010000006b483045022100994d614b679eed66fc5b0a2a85dcf0d993a6a8575b8af5fc164ff31015ae4a680220215badf69a74365046504141a1f2b1ce85b7280efd98fcf0a182ab8be999925c012102add319140c528a8955d76d4afe32c4d3143fea57ea353a31ce793cffb77ef861fdffffff0200e1f50500000000160014b500f5f6ce6b3aa7f5a871c49ab08ff38b451501b0fa6459000000001976a9142b19bade75a48768a5ffc142a86490303a95f41388ac00000000000000",
                "70736274ff0100710200000001c6a7269431c21132ad0db2c46ce7a71a79df57d9201f31fe46ee1b3d82f71ccf0000000000ffffffff0250c3000000000000160014622c23eebbf46df254d7da8e1c4d95d4f5c7d69f961cf50500000000160014b500f5f6ce6b3aa7f5a871c49ab08ff38b45150100000000000100df0100000001ce19e37c126c555f00d1af024e95dd7c7681d42831d27388897456bcf38bdff5010000006b483045022100994d614b679eed66fc5b0a2a85dcf0d993a6a8575b8af5fc164ff31015ae4a680220215badf69a74365046504141a1f2b1ce85b7280efd98fcf0a182ab8be999925c012102add319140c528a8955d76d4afe32c4d3143fea57ea353a31ce793cffb77ef861fdffffff0200e1f50500000000160014b500f5f6ce6b3aa7f5a871c49ab08ff38b451501b0fa6459000000001976a9142b19bade75a48768a5ffc142a86490303a95f41388ac00000000000000",
                "mymintaddress",
                "eth"
            )
            .await
            .expect("To setup new bridge lock");

        let psbt_event = attestor
            .get_psbt_event("testuuid".to_string())
            .await
            .expect("To get psbt event");

        let psbt_event: ApiOraclePsbtEvent =
            serde_wasm_bindgen::from_value(psbt_event).expect("To deserialize psbt event");

        assert_eq!(psbt_event.uuid, "testuuid".to_string());

        let psbt_events = attestor
            .get_psbt_events()
            .await
            .expect("To get psbt events");

        let _psbt_events: Vec<ApiOraclePsbtEvent> =
            serde_wasm_bindgen::from_value(psbt_events).expect("To deserialize psbt events");
    }
}
