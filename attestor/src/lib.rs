#[macro_use]
extern crate log;
extern crate core;
use ::hex::ToHex;
use wasm_bindgen::prelude::*;

use lightning::util::ser::{Readable, Writeable};

use secp256k1_zkp::rand::{self, thread_rng};
use secp256k1_zkp::{
    hashes::*, All, KeyPair, Message, Secp256k1, SecretKey, XOnlyPublicKey as SchnorrPublicKey,
};
use std::io::Cursor;

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
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub struct Attestor {
    oracle: Oracle,
}

#[wasm_bindgen]
impl Attestor {
    pub async fn new() -> Attestor {
        let secp = Secp256k1::new();
        let new_key = secp.generate_keypair(&mut rand::thread_rng()).0;
        let key_pair = KeyPair::from_secret_key(&secp, &new_key);
        let oracle = Oracle::new(key_pair, secp).unwrap();
        Attestor { oracle }
        // Attestor {}
    }

    pub async fn create_event(&mut self, uuid: &str, maturation: &str) -> Result<(), JsValue> {
        let maturation = OffsetDateTime::parse(maturation, &Rfc3339)
            .map_err(AttestorError::DatetimeParseError)
            .unwrap();

        clog!(
            "Creating event for uuid:{} and maturation_time :{}",
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
        if self.oracle.event_handler.storage_api.is_some() {
            self.oracle
                .event_handler
                .storage_api
                .as_ref()
                .unwrap()
                .insert(uuid.to_string(), new_event.clone())
                .await
                .unwrap();
        } else {
            self.oracle
                .event_handler
                .memory_api
                .as_mut()
                .unwrap()
                .insert(uuid.to_string(), new_event.clone())
                .await
                .unwrap();
        }

        Ok(())
    }

    pub async fn attest(&mut self, uuid: String, outcome: u64) {
        clog!("retrieving oracle event with uuid {}", uuid);
        let mut event: DbValue;
        if self.oracle.event_handler.storage_api.is_some() {
            let event_vec = match self
                .oracle
                .event_handler
                .storage_api
                .as_ref()
                .unwrap()
                .get(uuid.clone())
                .await
                .unwrap()
            {
                Some(val) => val,
                None => panic!(), // None => return Err(AttestorError::OracleEventNotFoundError(uuid).into()),
            };
            event = serde_json::from_str(&String::from_utf8_lossy(&event_vec)).unwrap();
        } else {
            let event_ivec = match self
                .oracle
                .event_handler
                .memory_api
                .as_ref()
                .unwrap()
                .get(uuid.clone())
                .await
                .unwrap()
            {
                Some(val) => val,
                // None => return Err(AttestorError::OracleEventNotFoundError(uuid).into()),
                None => panic!(),
            };
            event = serde_json::from_str(&String::from_utf8_lossy(&event_ivec)).unwrap();
        }

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
            &self.oracle.get_secp(),
            outcomes,
        );

        event.3 = Some(outcome);
        event.2 = Some(attestation.encode());

        let new_event = serde_json::to_string(&event).unwrap().into_bytes();

        if self.oracle.event_handler.storage_api.is_some() {
            let _insert_event = match self
                .oracle
                .event_handler
                .storage_api
                .as_ref()
                .unwrap()
                .insert(uuid, new_event.clone())
                .await
                .unwrap()
            {
                Some(val) => val,
                // None => return Err(AttestorError::OracleEventNotFoundError(uuid).into()),
                None => panic!(),
            };
        } else {
            let _insert_event = match self
                .oracle
                .event_handler
                .memory_api
                .as_mut()
                .unwrap()
                .insert(uuid, new_event.clone())
                .await
                .unwrap()
            {
                Some(val) => val,
                // None => return Err(AttestorError::OracleEventNotFoundError(uuid).into()),
                None => panic!(),
            };
        }
    }

    pub async fn get_events(&self) -> JsValue {
        let events = match self.oracle.event_handler.storage_api {
            Some(ref storage_api) => storage_api.get_all().await.unwrap().unwrap(),
            None => self
                .oracle
                .event_handler
                .memory_api
                .as_ref()
                .unwrap()
                .get_all()
                .await
                .unwrap()
                .unwrap(),
        };

        let events: Vec<ApiOracleEvent> = events
            .iter()
            .map(|event| parse_database_entry(event.clone().1.into()))
            .collect();

        serde_wasm_bindgen::to_value(&events).unwrap()
    }

    pub async fn get_event(&self, uuid: String) -> JsValue {
        let result = match self.oracle.event_handler.storage_api {
            Some(ref storage_api) => storage_api.get(uuid).await.unwrap(),
            None => self
                .oracle
                .event_handler
                .memory_api
                .as_ref()
                .unwrap()
                .get(uuid)
                .await
                .unwrap(),
        };

        match result {
            Some(event) => {
                serde_wasm_bindgen::to_value(&parse_database_entry(event.into())).unwrap()
            }
            None => JsValue::NULL,
        }
    }

    pub async fn get_pubkey(&self) -> String {
        let pubkey = SchnorrPublicKey::from_keypair(&self.oracle.key_pair).0;
        pubkey.to_string()
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
                Err(_) => Some("Error decoding attestatoin".to_string()),
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
        .expect("Error writing oracle event");
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
