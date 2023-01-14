use crate::OracleConfig;
use secp256k1_zkp::All;
use secp256k1_zkp::KeyPair;
use secp256k1_zkp::Secp256k1;
use serde::{Deserialize, Serialize};

mod error;
mod handler;
use crate::oracle::handler::EventHandler;
pub use error::OracleError;
pub use error::Result;

#[derive(Clone, Deserialize, Serialize)]
// outstanding_sk_nonces?, suredbits_announcement, suredbits_attestation?, announcement, attestation?, outcome?, uuid
pub struct DbValue(
    pub Option<Vec<[u8; 32]>>,
    pub Vec<u8>,
    pub Option<Vec<u8>>,
    pub Vec<u8>,
    pub Option<Vec<u8>>,
    pub Option<u64>,
    pub String,
);

#[derive(Clone)]
pub struct Oracle {
    pub oracle_config: OracleConfig,
    pub event_handler: EventHandler,
    keypair: KeyPair,
    secp: Secp256k1<All>,
}

impl Oracle {
    pub fn new(
        oracle_config: OracleConfig,
        keypair: KeyPair,
        secp: Secp256k1<All>,
    ) -> Result<Oracle> {
        if !oracle_config.announcement_offset.is_positive() {
            return Err(OracleError::InvalidAnnouncementTimeError(
                oracle_config.announcement_offset,
            ));
        }
        let event_handler = EventHandler::new();

        Ok(Oracle {
            oracle_config,
            event_handler,
            keypair,
            secp,
        })
    }

    pub fn get_keypair(&self) -> &KeyPair {
        &self.keypair
    }
    pub fn get_secp(&self) -> &Secp256k1<All> {
        &self.secp
    }
}

pub mod oracle_queryable;
pub mod secret_key;
pub mod vault;

pub use oracle_queryable::messaging::EventDescriptor;
