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
    pub async fn new(storage_api_endpoint: String, x_secret_key_str: String) -> Attestor {
        clog!(
            "[WASM-ATTESTOR]: Creating new attestor with storage_api_endpoint: {}",
            storage_api_endpoint
        );
        let secp = Secp256k1::new();
        let xpriv_key = ExtendedPrivKey::from_str(&x_secret_key_str)
            .expect("Unable to decode xpriv env variable");
        let external_derivation_path =
            DerivationPath::from_str("m/44h/0h/0h/0").expect("A valid derivation path");
        let derived_ext_xpriv = xpriv_key
            .derive_priv(
                &secp,
                &external_derivation_path.extend([
                    ChildNumber::Normal { index: 0 },
                    ChildNumber::Normal { index: 0 },
                ]),
            )
            .expect("Should be able to derive the private key path during wallet setup");
        let secret_key = derived_ext_xpriv.private_key;
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let oracle = Oracle::new(key_pair, secp, storage_api_endpoint).unwrap();
        Attestor { oracle, secret_key }
    }

    pub async fn get_health() -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&json!({"data": [
            {"status": "healthy", "message": ""}
        ]}))?)
    }

    pub async fn create_event(&self, uuid: &str, maturation: &str) -> Result<(), JsValue> {
        let maturation = OffsetDateTime::parse(maturation, &Rfc3339)
            .map_err(AttestorError::DatetimeParseError)
            .unwrap();

        clog!(
            "[WASM-ATTESTOR] Creating event for uuid: {} and maturation_time : {}",
            uuid,
            maturation
        );

        let (announcement_obj, outstanding_sk_nonces) = build_announcement(
            &self.oracle.key_pair,
            &self.oracle.secp,
            maturation,
            uuid.to_string(),
        )
        .unwrap();

        let db_value = DbValue(
            Some(outstanding_sk_nonces),
            announcement_obj.encode(),
            None,
            None,
            uuid.to_string(),
        );

        let new_event = serde_json::to_string(&db_value).unwrap().into_bytes();

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

    pub async fn attest(&self, uuid: String, outcome: u64) {
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
                clog!(
                    "[WASM-ATTESTOR] Error retrieving event from StorageAPI: {:?}",
                    e
                );
                panic!();
            }
        };
        let event_vec = match res {
            Some(val) => val,
            None => {
                clog!(
                    "[WASM-ATTESTOR] Event missing in StorageAPI with uuid: {}",
                    uuid
                );
                panic!();
            }
        };
        event = serde_json::from_str(&String::from_utf8_lossy(&event_vec)).unwrap();

        let outstanding_sk_nonces = event.clone().0.unwrap();

        let announcement = OracleAnnouncement::read(&mut Cursor::new(&event.1)).unwrap();

        let num_digits_to_sign = match announcement.oracle_event.event_descriptor {
            dlc_messages::oracle_msgs::EventDescriptor::DigitDecompositionEvent(e) => e.nb_digits,
            _ => {
                panic!()
                // return Err(AttestorError::OracleEventNotFoundError(
                //     "Got an unexpected EventDescriptor type!".to_string(),
                // )
                // .into())
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

        let new_event = serde_json::to_string(&event).unwrap().into_bytes();

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
    }

    pub async fn get_events(&self) -> JsValue {
        let events = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get_all(self.secret_key)
            .await
            .unwrap()
            .unwrap();

        let events: Vec<ApiOracleEvent> = events
            .iter()
            .map(|event| parse_database_entry(event.clone().1))
            .collect();

        serde_wasm_bindgen::to_value(&events).unwrap()
    }

    pub async fn get_event(&self, uuid: String) -> JsValue {
        let result = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get(uuid, self.secret_key)
            .await
            .unwrap();

        match result {
            Some(event) => serde_wasm_bindgen::to_value(&parse_database_entry(event)).unwrap(),
            None => JsValue::NULL,
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

#[derive(Serialize)]
struct ApiOracleEvent {
    event_id: String,
    uuid: String,
    rust_announcement_json: String,
    rust_announcement: String,
    rust_attestation_json: Option<String>,
    rust_attestation: Option<String>,
    maturation: String,
    outcome: Option<u64>,
}

fn parse_database_entry(event: Vec<u8>) -> ApiOracleEvent {
    let event: DbValue = serde_json::from_str(&String::from_utf8_lossy(&event)).unwrap();

    let announcement_vec = event.1.clone();
    let announcement = OracleAnnouncement::read(&mut Cursor::new(&announcement_vec)).unwrap();

    let db_att = event.2.clone();
    let decoded_att_json = match db_att {
        None => None,
        Some(att_vec) => {
            let mut attestation_cursor = Cursor::new(&att_vec);

            match OracleAttestation::read(&mut attestation_cursor) {
                Ok(att) => Some(format!("{:?}", att)),
                Err(_) => Some("[WASM-ATTESTOR] Error decoding attestatoin".to_string()),
            }
        }
    };

    ApiOracleEvent {
        event_id: announcement.oracle_event.event_id.clone(),
        uuid: event.4,
        rust_announcement_json: serde_json::to_string(&announcement).unwrap(),
        rust_announcement: event.1.encode_hex::<String>(),
        rust_attestation_json: decoded_att_json,
        rust_attestation: event.2.map(|att| att.encode_hex::<String>()),
        maturation: announcement.oracle_event.event_maturity_epoch.to_string(),
        outcome: event.3,
    }
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
        .map(|x| KeyPair::from_seckey_slice(secp, x.as_ref()).unwrap())
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
        event_maturity_epoch: maturation.unix_timestamp().try_into().unwrap(),
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
