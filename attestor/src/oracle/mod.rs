use bitcoin::hashes::hex::FromHex;
use bitcoin::psbt::PartiallySignedTransaction;
use secp256k1_zkp::PublicKey;
use secp256k1_zkp::{All, KeyPair, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};

pub(crate) mod error;
mod handler;
use crate::oracle::handler::EventHandler;
use crate::PsbtEventStatus;
pub use error::OracleError;
// pub use error::Result;

use self::error::GenericOracleError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DbValue(
    pub Option<Vec<SecretKey>>,           // outstanding_sk_nonces?
    pub Vec<u8>,                          // announcement
    pub Option<Vec<u8>>,                  // attestation?
    pub Option<u64>,                      // outcome?
    pub String,                           // uuid
    #[serde(default)] pub Option<String>, // chain name
);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PsbtEvent {
    pub closing_psbt: String,    // closing_psbt as hex of byte array
    pub mint_address: String,    // corresponding mint address
    pub uuid: String,            // uuid
    pub funding_txid: String,    // funding_txid
    pub outcome: Option<u64>,    // outcome?
    pub status: PsbtEventStatus, // status
    #[serde(default)]
    pub chain_name: Option<String>, // chain_name?
}

// impl From<ApiOraclePsbtEvent> for PsbtEvent {
//     fn from(psbt_event: ApiOraclePsbtEvent) -> Self {
//         PsbtEvent {
//             closing_psbt: bitcoin::consensus::encode::serialize(&psbt_event.closing_psbt).to_hex(),
//             mint_address: psbt_event.mint_address,
//             uuid: psbt_event.uuid,
//             funding_txid: psbt_event.funding_txid,
//             outcome: psbt_event.outcome,
//             status: psbt_event.status,
//             chain_name: psbt_event.chain,
//         }
//     }
// }

impl PsbtEvent {
    pub fn deserialize(bytes: &[u8]) -> Result<PsbtEvent, GenericOracleError> {
        serde_json::from_slice(bytes).map_err(|e| GenericOracleError {
            message: format!(
                "[WASM-ATTESTOR] Error deserializing psbt events from JSON: {}",
                e
            ),
        })
    }

    // pub fn serialize(&self) -> Result<Vec<u8>, GenericOracleError> {
    //     Ok(serde_json::to_string(self)
    //         .map_err(|e| GenericOracleError {
    //             message: format!(
    //                 "[WASM-ATTESTOR] Error serializing psbt events to JSON: {}",
    //                 e
    //             ),
    //         })?
    //         .into_bytes())
    // }

    pub fn get_closing_psbt(
        &self,
    ) -> Result<bitcoin::util::psbt::PartiallySignedTransaction, GenericOracleError> {
        let closing_psbt: Vec<u8> =
            FromHex::from_hex(&self.closing_psbt).map_err(|e| GenericOracleError {
                message: format!(
                    "[WASM-ATTESTOR] Error serializing psbt event to JSON: {}",
                    e
                ),
            })?;
        let closing_psbt: PartiallySignedTransaction =
            bitcoin::consensus::deserialize(&closing_psbt).map_err(|e| GenericOracleError {
                message: format!("Unable to deserialize closing psbt event from db: {}", e),
            })?;
        Ok(closing_psbt)
    }
}

#[derive(Clone)]
pub struct Oracle {
    pub event_handler: EventHandler,
    pub key_pair: KeyPair,
    pub secp: Secp256k1<All>,
}

impl Oracle {
    pub fn new(
        key_pair: KeyPair,
        secp: Secp256k1<All>,
        storage_api_endpoint: String,
    ) -> Result<Oracle, OracleError> {
        let event_handler = EventHandler::new(
            storage_api_endpoint,
            PublicKey::from_keypair(&key_pair).to_string(),
        );

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
