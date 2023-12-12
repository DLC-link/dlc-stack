#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]

extern crate core;
extern crate log;
use ::hex::ToHex;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
use serde_json::json;
use wasm_bindgen::prelude::*;

use lightning::util::ser::{Readable, Writeable};

use secp256k1_zkp::rand::thread_rng;
use secp256k1_zkp::{
    hashes::*, All, KeyPair, Message, Secp256k1, SecretKey, XOnlyPublicKey as SchnorrPublicKey,
};
use std::io::Cursor;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod oracle;
use oracle::Oracle;

use oracle::DbValue;

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
        rust_announcement: event.1.encode_hex::<String>(),
        rust_attestation_json: decoded_att_json,
        rust_attestation: event.2.map(|att| att.encode_hex::<String>()),
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
