use secp256k1_zkp::{All, KeyPair, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};

mod error;
mod handler;
use crate::oracle::handler::EventHandler;
pub use error::OracleError;
pub use error::Result;

#[derive(Clone, Deserialize, Serialize)]
// outstanding_sk_nonces?, suredbits_announcement, suredbits_attestation?, announcement, attestation?, outcome?, uuid
pub struct DbValue(
    pub Option<Vec<SecretKey>>,
    pub Vec<u8>,
    pub Option<Vec<u8>>,
    pub Vec<u8>,
    pub Option<Vec<u8>>,
    pub Option<u64>,
    pub String,
);

#[derive(Clone)]
pub struct Oracle {
    pub event_handler: EventHandler,
    pub key_pair: KeyPair,
    pub secp: Secp256k1<All>,
}

impl Oracle {
    pub fn new(key_pair: KeyPair, secp: Secp256k1<All>) -> Result<Oracle> {
        let event_handler = EventHandler::new();

        Ok(Oracle {
            event_handler,
            key_pair,
            secp,
        })
    }

    pub fn get_keypair(&self) -> &KeyPair {
        &self.key_pair
    }
    pub fn get_secp(&self) -> &Secp256k1<All> {
        &self.secp
    }
}

// pub mod oracle_queryable;
pub mod secret_key;
pub mod vault;
