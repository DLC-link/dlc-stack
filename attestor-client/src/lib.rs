//! # cg-oracle-client
//! Http client wrapper for the Crypto Garage DLC oracle
#![feature(async_fn_in_trait)]
// Coding conventions
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
// #![deny(unused_mut)]
// #![deny(dead_code)]
#![deny(unused_imports)]
// #![deny(missing_docs)]

extern crate chrono;
extern crate dlc_manager;
extern crate dlc_messages;
extern crate secp256k1_zkp;
extern crate serde;

use std::{fmt, io::Cursor, num::ParseIntError, time::Duration};

use chrono::{DateTime, Utc};
use dlc_link_manager::AsyncOracle;
use dlc_manager::error::Error as DlcManagerError;
use dlc_messages::oracle_msgs::{OracleAnnouncement, OracleAttestation};
use log::info;
use secp256k1_zkp::{schnorr::Signature, XOnlyPublicKey};
use serde_json::Value;

const REQWEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Enables interacting with a DLC oracle.
pub struct AttestorClient {
    host: String,
    public_key: XOnlyPublicKey,
    // client: reqwest::blocking::Client,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicKeyResponse {
    public_key: XOnlyPublicKey,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EventDescriptor {
    base: u16,
    is_signed: bool,
    unit: String,
    precision: i32,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    nonces: Vec<XOnlyPublicKey>,
    event_maturity: DateTime<Utc>,
    event_id: String,
    event_descriptor: EventDescriptor,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnoucementResponse {
    oracle_public_key: XOnlyPublicKey,
    oracle_event: Event,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AttestationResponse {
    event_id: String,
    signatures: Vec<Signature>,
    values: Vec<String>,
}

// fn get_object<T>(path: &str) -> Result<T, DlcManagerError>
// where
//     T: serde::de::DeserializeOwned,
// {
//     reqwest::blocking::get(path)
//         .map_err(|x| {
//             dlc_manager::error::Error::IOError(std::io::Error::new(std::io::ErrorKind::Other, x))
//         })?
//         .json::<T>()
//         .map_err(|e| dlc_manager::error::Error::OracleError(e.to_string()))
// }

async fn get_json(path: &str) -> Result<Value, DlcManagerError> {
    let mut client_builder = reqwest::ClientBuilder::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        client_builder = client_builder.timeout(REQWEST_TIMEOUT);
    }
    client_builder
        .build()
        .map_err(|e| {
            DlcManagerError::IOError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                e.to_string(),
            ))
        })?
        .get(path)
        .send()
        .await
        .map_err(|x| {
            dlc_manager::error::Error::IOError(std::io::Error::new(std::io::ErrorKind::Other, x))
        })?
        .json::<Value>()
        .await
        .map_err(|x| {
            dlc_manager::error::Error::IOError(std::io::Error::new(std::io::ErrorKind::Other, x))
        })
}

fn pubkey_path(host: &str) -> String {
    format!("{}{}", host, "publickey")
}

fn announcement_path(host: &str, event_id: &str) -> String {
    format!("{}event/{}", host, event_id)
}

fn attestation_path(host: &str, event_id: &str) -> String {
    format!("{}event/{}", host, event_id)
}

impl AttestorClient {
    /// Try to create an instance of an oracle client connecting to the provided
    /// host. Returns an error if the host could not be reached. Panics if the
    /// oracle uses an incompatible format.
    #[allow(dead_code)]
    pub async fn new(host: &str) -> Result<AttestorClient, DlcManagerError> {
        if host.is_empty() {
            return Err(DlcManagerError::InvalidParameters(
                "Invalid host".to_string(),
            ));
        }
        let host = if !host.ends_with('/') {
            format!("{}{}", host, "/")
        } else {
            host.to_string()
        };
        info!("Creating p2pd oracle client (by getting public key first) ...");
        let path = pubkey_path(&host);
        info!("Getting pubkey from {}", path);

        let mut client_builder = reqwest::ClientBuilder::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            client_builder = client_builder.timeout(REQWEST_TIMEOUT);
        }
        let attestor_key = client_builder
            .build()
            .map_err(|e| {
                DlcManagerError::IOError(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    e.to_string(),
                ))
            })?
            .get(path)
            .send()
            .await
            .map_err(|e| DlcManagerError::OracleError(format!("Oracle PubKey Error: {e}")))?
            .text()
            .await
            .map_err(|e| DlcManagerError::OracleError(format!("Oracle PubKey Error: {e}")))?;

        info!("Attestor Pub Key: {}", attestor_key.to_string());

        let public_key: XOnlyPublicKey = attestor_key
            .parse()
            .map_err(|_| DlcManagerError::OracleError("Oracle PubKey Error".to_string()))?;
        info!("The p2pd oracle client has been created successfully");
        Ok(AttestorClient { host, public_key })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeHexError {
    OddLength,
    ParseInt(ParseIntError),
}

impl From<ParseIntError> for DecodeHexError {
    fn from(e: ParseIntError) -> Self {
        DecodeHexError::ParseInt(e)
    }
}

impl fmt::Display for DecodeHexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeHexError::OddLength => "input string has an odd number of bytes".fmt(f),
            DecodeHexError::ParseInt(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for DecodeHexError {}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, DecodeHexError> {
    if s.len() % 2 != 0 {
        Err(DecodeHexError::OddLength)
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.into()))
            .collect()
    }
}

impl AsyncOracle for AttestorClient {
    async fn get_public_key(&self) -> XOnlyPublicKey {
        self.public_key
    }

    async fn get_announcement(
        &self,
        event_id: &str,
    ) -> Result<OracleAnnouncement, DlcManagerError> {
        info!("Getting announcement for event_id {event_id}");
        let path = announcement_path(&self.host, event_id);
        info!("Getting announcement at URL {path}");
        let v = match get_json(&path).await {
            Ok(v) => v,
            Err(e) => {
                return Err(DlcManagerError::OracleError(format!(
                    "Error getting announcement: {e}",
                    e = e
                )))
            }
        };

        let encoded_hex_announcement = match v["rust_announcement"].as_str() {
            //call to_string instead of as_str and watch your world crumble to pieces
            None => {
                return Err(DlcManagerError::OracleError(format!(
                    "missing announcement for event {}",
                    event_id,
                )))
            }
            Some(s) => s,
        };

        let buffer = decode_hex(encoded_hex_announcement)
            .map_err(|e| DlcManagerError::OracleError(format!("Error decoding hex: {e}", e = e)))?;

        let mut announcement_cursor = Cursor::new(buffer);
        let decoded_announcement =
        <dlc_messages::oracle_msgs::OracleAnnouncement as lightning::util::ser::Readable>::read(
            &mut announcement_cursor,
        )
        .unwrap();

        Ok(decoded_announcement)
    }

    async fn get_attestation(
        &self,
        event_id: &str,
    ) -> Result<OracleAttestation, dlc_manager::error::Error> {
        let path = attestation_path(&self.host, event_id);
        let v = get_json(&path).await?;

        //TODO: this next line might be None, throwing at unwrap, fix
        let encoded_hex_attestation = match v["rust_attestation"].as_str() {
            None => {
                return Err(DlcManagerError::OracleError(format!(
                    "missing attestation for event {}",
                    event_id,
                )))
            }
            Some(s) => s,
        };

        let buffer = decode_hex(encoded_hex_attestation).unwrap();

        let mut attestation_cursor = Cursor::new(buffer);
        let decoded_attestation =
            <dlc_messages::oracle_msgs::OracleAttestation as lightning::util::ser::Readable>::read(
                &mut attestation_cursor,
            )
            .unwrap();

        info!("GOT ATTESTATION as OBJECT! {:?}", decoded_attestation);
        Ok(decoded_attestation)
    }
}

// #[cfg(test)]
// mod tests {
//     extern crate mockito;
//     use self::mockito::{mock, Mock};
//     use super::*;

//     fn pubkey_mock() -> Mock {
//         let path: &str = &pubkey_path("/");
//         mock("GET", path)
//             .with_body(r#""ce4b7ad2b45de01f0897aa716f67b4c2f596e54506431e693f898712fe7e9bf3""#)
//             .create()
//     }

//     #[test]
//     fn get_public_key_test() {
//         let url = &mockito::server_url();
//         let _pubkey_mock = pubkey_mock();
//         let expected_pk: XOnlyPublicKey =
//             "ce4b7ad2b45de01f0897aa716f67b4c2f596e54506431e693f898712fe7e9bf3"
//                 .parse()
//                 .unwrap();

//         let client = AttestorClient::new(url).expect("Error creating client instance.");

//         assert_eq!(expected_pk, client.get_public_key());
//     }

//     #[test]
//     fn get_announcement_test() {
//         let url = &mockito::server_url();
//         let _pubkey_mock = pubkey_mock();
//         let path: &str = &announcement_path("/", "uniqueeventid123");
//         let _m = mock("GET", path).with_body(r#"{"event_id": "uniqueeventid123", "rust_announcement": "850d0f8cba638f2902dd75ba47dbc68662356249414ae90f5f3b0e8b85663801152045b10bad679c9a02ed627a33601f61ad90d97b76f1b78155192918e42d1a57c75f44e4dde2a17c0da725c160267e3307d771a33685dcd67ec79c7614722cfdd822fd021d000e2025053441f55e0096864bba15d3ab891f1a232a8518c06ea1ac745fc7fa13fc34f4f18e5fc392db419d0bd38f47b76a0f86ac60aaa67c426804a59c8ac214ce7a2f1c67e301d87f42575582e972f4a8b16f11236ad88630049fd96c9f711ce4c62b09d89ea422b696c366a14670908c229f9c5d4c43904ff5cd5ea096e6f35d08665144ef2dbacf4a6a9fadf3075a776d202ddd491ee85afd24bc4b47e2f23918a76ebb71f473575da7f2436ca5a62482676939bb74b7e3ebb0fdf70deadf9d51c403e606827b304dd65517786bcd08345d8db9885af942b4b71e60aa18175ace4035cc5c5480b3199c02691e9d411af63f6c15e9cabbecfea8f91c001e45019e29fe0e3c745a3c531a78f2f7e393a5f3a4f7179359c8cb6a8cce976a750ab98063b4eef7a4e131978c4d3d7bfccde62b7fd76f168d446095f24da149010d905e39eec4f3b455c91a72b5554710e452a0b589cb0e1a9c96b512e0aa4a21b3f4c8e2c124685067292527a74955d15f1df9909df53f7078a41c50778052f1f0aed958559fefde8dd391184ed71d069aa90663b419b9be889b8aecceedd5bde3c395553247438ccb277897493ca90c71f4b37925e565614374d7353e7c4f1d3bbb00000005fdd80a100002000642544355534400000000000e42307830306461633663393661616438666232666637376537613666313266626236646237346235333930623534343330613232393230633139373862336538336432"}"#).create();

//         let client = AttestorClient::new(url).expect("Error creating client instance");

//         client
//             .get_announcement("uniqueeventid123")
//             .expect("Error getting announcement");
//     }

//     #[test]
//     fn get_attestation_test() {
//         let url = &mockito::server_url();
//         let _pubkey_mock = pubkey_mock();
//         let path: &str = &attestation_path("/", "uniqueeventid123");

//         let _m = mock("GET", path).with_body(r#"{"event_id": "uniqueeventid123", "rust_attestation": "57c75f44e4dde2a17c0da725c160267e3307d771a33685dcd67ec79c7614722c000e435c1226b39a15c5c838bfede48e3991a7f635fc126b4d03818309967679f8ce6caf640dd446d196e2ce7d42309737ec6c9c7deeb2d2342f7d136d1adc47d8cbdbbf9310b26fa8d0e9bf1358c805398d0c0544982b86df7cea226ba3d90df5187e7ae34c2e9d4287388818702198a76a5e64a7542f47bb47e394122ca82a013d9b9f576b113886df98924027471624157cda05ac1e591d939233bea95ff858c1f07d0dde3e7b9eac58d3f0bee20c8a3883238656a68711571fc4f3394fd447c090d0aa53d0a79c066e98f3f7e8069b0e4fab53e65003ce09e75591815414b976d85b3b54620a006f26a756530b4380cdd3d7eb9624aa82ef81b613f06a97228194364a717f0ad64de704a1492afbe24fb58f9dcb60af9bbc4d493bec5dccf559b80cb34f313a7be4224d8ca40eaf38f0a2a2cb37bff478fe95467d8861dfd124945eb1a55c0b891b7adba33c2ce682b392b588634d7dd5a0304a16ded6de705af196897ce28b517b7f00d42c3e8220fc9b06d979131e93cce043e88281dc904fbebfa4d2e0612f9c3c612f974e582d282b944a026db59850a3b6d58e5d70a8eb555382b1300c4639471aa7c6d534a495e69ab98382225baa81e88ec7c849667529468f5534e1e25e984a53ad90a84063a10c21cf82d80fed7630db3b76a7c9a7ae633e64f3cbf9b78d5e43b5c2397d75c014c23c5c3362b87065044d5a4be86e0e6cd27c97dc188477821ec2ad5bf4fc8801570342003d05ba2aaa8dffb28882103121b6dd9d067dc8beba74cdec23bc2f97f0b9d650c745fc619e6d9b9196754894f78a9c0d7389e6dba483839b9a481673b84c965b1c3c1eb206a0fffc46e9b7298d08a0cd0453d1009e75a52eeffb2b46cf67f1b11cda16f83a1ec84ee57db6e9cd7011dcf24f378117ccf72f2fdae2db8f701fcfdf957b8ce3264493b9e20af28e009962bf5741139d2c95631e70105aecb53ad1a7fce3945a9a8e61e766eb98082e6be726c0a0d0c3f53faa550fc25188f1d65dd73df394de5929c223b4a952ed3f41dbb669d78b094283e5a62e20aac76aef9d352be841e5ded6e89a5dc662a54667ec91234aab02a06db5a68c689da223b152cfc4bceb03d1097322ec1c6672a5fd562b79892a1a4eaa1f4b2850ad4dc41427ced86f509e414ce915f54c51b3734bb11f28a7bd39afb2f04775b483a8a465faf4eee5449ff63cbbcfd7d22dd0c22d6a4dd49e3d8b96c37b5736bf0a4f9472d85552c13682a9d482bdf3000e01300130013001300130013001300130013001300130013001300130"}"#).create();

//         let client = AttestorClient::new(url).expect("Error creating client instance");

//         client
//             .get_attestation("uniqueeventid123")
//             .expect("Error getting attestation");
//     }
// }
