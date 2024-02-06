#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]

extern crate core;
extern crate log;

use bdk::blockchain::EsploraBlockchain;
use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::{Builder, Instruction};
use bitcoin::consensus::deserialize;
use bitcoin::hashes::hex::{FromHex, ToHex};
use bitcoin::psbt::{Input, PartiallySignedTransaction, Prevouts, TapTree};
use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
use bitcoin::util::key::XOnlyPublicKey;
use bitcoin::util::sighash::{ScriptPath, SighashCache};
use bitcoin::util::taproot::{ControlBlock, LeafVersion, TapTweakHash, TaprootBuilder};
use bitcoin::{SchnorrSig, SchnorrSighashType, Script, Witness};
use serde_json::json;
use wasm_bindgen::prelude::*;

use lightning::util::ser::{Readable, Writeable};

use secp256k1_zkp::rand::thread_rng;
use secp256k1_zkp::{
    hashes::*, All, KeyPair, Message, Secp256k1, SecretKey, XOnlyPublicKey as SchnorrPublicKey,
};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::str::FromStr;

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
    ) -> Result<Attestor, JsError> {
        clog!(
            "[WASM-ATTESTOR]: Creating new attestor with storage_api_endpoint: {}",
            storage_api_endpoint
        );
        let secp = Secp256k1::new();
        let xpriv_key = ExtendedPrivKey::from_str(&x_secret_key_str)
            .map_err(|_| JsError::new("Unable to decode xpriv env variable"))?;
        let external_derivation_path = DerivationPath::from_str("m/44h/0h/0h/0")
            .map_err(|_| JsError::new("A valid derivation path"))?;
        let derived_ext_xpriv = xpriv_key
            .derive_priv(
                &secp,
                &external_derivation_path.extend([
                    ChildNumber::Normal { index: 0 },
                    ChildNumber::Normal { index: 0 },
                ]),
            )
            .map_err(|_| {
                JsError::new("Should be able to derive the private key path during wallet setup")
            })?;
        let secret_key = derived_ext_xpriv.private_key;
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let oracle = Oracle::new(key_pair, secp, storage_api_endpoint)
            .map_err(|_| JsError::new("Error creating Oracle"))?;
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
    ) -> Result<(), JsError> {
        let maturation = OffsetDateTime::parse(maturation, &Rfc3339)
            .map_err(|_| JsError::new("Unable to parse maturation time"))?;

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
        .map_err(|_| JsError::new("Error building announcement"))?;

        let db_value = DbValue(
            Some(outstanding_sk_nonces),
            announcement_obj.encode(),
            None,
            None,
            uuid.to_string(),
            Some(chain.to_string()),
        );

        let new_event = serde_json::to_string(&db_value)
            .map_err(|_| JsError::new("Error serializing new_event to JSON"))?
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
                Err(JsError::new("Failed to create event"))
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

    pub async fn get_events(&self) -> Result<JsValue, JsError> {
        let events = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get_all(self.secret_key)
            .await
            .map_err(|_| JsError::new("[WASM-ATTESTOR] Error getting all events"))?;

        let events: Vec<ApiOracleEvent> = events
            .iter()
            .filter_map(|event| parse_database_entry(event.clone().1).ok())
            .collect();

        serde_wasm_bindgen::to_value(&events)
            .map_err(|_| JsError::new("[WASM-ATTESTOR] Error serializing events to JSON"))
    }

    pub async fn get_event(&self, uuid: String) -> Result<JsValue, JsError> {
        let result = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get(uuid, self.secret_key)
            .await
            .map_err(|_| JsError::new("[WASM-ATTESTOR] Error getting event"))?;

        match result {
            Some(event) => {
                let parsed_event = parse_database_entry(event)
                    .map_err(|_| JsError::new("[WASM-ATTESTOR] Error parsing database entry"))?;
                serde_wasm_bindgen::to_value(&parsed_event)
                    .map_err(|_| JsError::new("[WASM-ATTESTOR] Error serializing event to JSON"))
            }
            None => Ok(JsValue::NULL),
        }
    }

    pub async fn get_pubkey(&self) -> String {
        SchnorrPublicKey::from_keypair(&self.oracle.key_pair)
            .0
            .to_string()
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

    async fn get_all_psbt_events(&self) -> Result<Vec<ApiOraclePsbtEvent>, GenericOracleError> {
        let psbt_events = self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .get_all(self.secret_key)
            .await
            .map_err(|_| GenericOracleError {
                message: "Error getting event from storage_api".to_string(),
            })?;

        let events: Vec<ApiOraclePsbtEvent> = psbt_events
            .iter()
            .filter_map(|event| parse_psbt_database_entry(event.clone().1).ok())
            .collect();

        Ok(events)
    }

    pub async fn create_psbt_event(
        &self,
        uuid: &str, // can use the txid of the prefunding tx here
        closing_psbt: &str,
        mint_address: &str,
        chain: &str,
    ) -> Result<(), JsError> {
        clog!(
            "[WASM-ATTESTOR] Creating new psbt event with uuid: {}",
            uuid
        );
        let closing_psbt: Vec<u8> = FromHex::from_hex(closing_psbt)
            .map_err(|_| JsError::new("Error decoding closing_psbt hex"))?;
        let closing_psbt: PartiallySignedTransaction = deserialize(&closing_psbt[..])
            .map_err(|_| JsError::new("Error decoding closing_psbt"))?;

        let fuding_txid = closing_psbt.clone().extract_tx().input[0]
            .previous_output
            .txid
            .to_string();

        let psbt_db_value = PsbtDbValue(
            bitcoin::consensus::encode::serialize(&closing_psbt).to_hex(),
            mint_address.to_string(),
            uuid.to_string(),
            fuding_txid,
            None,                     //outcome
            PsbtEventStatus::Pending, //status
            Some(chain.to_string()),  //chain name
        );

        let new_psbt_event = serde_json::to_string(&psbt_db_value)
            .map_err(|_| JsError::new("Error serializing new_event to JSON"))?
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
                Err(JsError::new("Failed to create psbt locking event"))
            }
        }
    }

    // Much of the logic of this function is taken from the following rust-bitcoin example:
    // https://github.com/rust-bitcoin/rust-bitcoin/issues/1195#issuecomment-1216163286
    // here's how we can put the hash-preimage into the witness to spend the htlc
    // https://github.com/lightningdevkit/rust-lightning/blob/main/lightning/src/ln/chan_utils.rs#L712
    // https://github.com/rust-bitcoin/rust-bitcoin/blob/master/bitcoin/examples/taproot-psbt.rs
    // https://github.com/rust-bitcoin/rust-bitcoin/blob/master/bitcoin/examples/taproot-psbt.rs#L686
    pub async fn close_psbt_event(&self, uuid: &str) -> Result<JsValue, JsError> {
        // reference https://github.com/paulmillr/scure-btc-signer/blob/87989df23ed931fa6fa9aeb4391c7c8c6bae53f3/index.ts#L1213
        // const TAPROOT_UNSPENDABLE_KEY: PublicKey = sha256(ProjPoint.BASE.toRawBytes(false));
        const TAPROOT_UNSPENDABLE_KEY: &str =
            "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

        clog!("[WASM-ATTESTOR] Closing psbt event with uuid: {}", uuid);

        let psbt_event = match self.get_psbt_event(uuid.to_string()).await {
            Ok(psbt_event) => psbt_event,
            Err(e) => {
                clog!(
                    "[WASM-ATTESTOR] Error getting psbt event with uuid: {}, error: {:?}",
                    uuid,
                    e
                );
                return Err(JsError::new(&format!(
                    "Error getting psbt event, event with uuid not found: {}",
                    uuid,
                )));
            }
        };
        let closing_psbt: PartiallySignedTransaction = psbt_event.clone().closing_psbt;

        // Starting the closing flow of signing and broadcasting the closing transaction
        let secp = Secp256k1::new();
        let attestor_keypair = KeyPair::from_secret_key(&secp, &self.secret_key);
        let attestor_xonlypubkey = SchnorrPublicKey::from_keypair(&attestor_keypair).0;

        let alice_xonlypubkey = closing_psbt.inputs[0]
            .tap_script_sigs
            .first_key_value()
            .unwrap_throw()
            .0
             .0;

        let psbt_script_instruction_set: Result<
            Vec<Instruction>,
            bitcoin::blockdata::script::Error,
        > = closing_psbt.inputs[0]
            .tap_scripts
            .first_key_value()
            .ok_or(JsError::new("Error getting tap_scripts"))?
            .1
             .0
            .instructions()
            .collect();

        let instructions = psbt_script_instruction_set.expect_throw("a valid instruction set");

        let alice_key_as_bytes = alice_xonlypubkey.clone().serialize();
        let attestor_key_as_bytes = attestor_xonlypubkey.clone().serialize();
        let first_instruction = instructions
            .first()
            .expect_throw(" a non empty instruction set")
            .to_owned();
        let alice_key_bytes = Instruction::PushBytes(&alice_key_as_bytes);
        let attestor_key_bytes = Instruction::PushBytes(&attestor_key_as_bytes);

        if ![attestor_key_bytes.clone(), alice_key_bytes.clone()].contains(&first_instruction) {
            return Err(JsError::new(
                "Error determining the order of the keys in the multisig script",
            ));
        }
        let is_attestor_key_first_in_multisig_script_order =
            first_instruction == attestor_key_bytes;

        let multisig_script_builder = Builder::new();
        let multisig_script = match is_attestor_key_first_in_multisig_script_order {
            true => multisig_script_builder
                .push_x_only_key(&attestor_xonlypubkey)
                .push_opcode(all::OP_CHECKSIGVERIFY)
                .push_x_only_key(&alice_xonlypubkey)
                .push_opcode(all::OP_CHECKSIG)
                .into_script(),
            false => multisig_script_builder
                .push_x_only_key(&alice_xonlypubkey)
                .push_opcode(all::OP_CHECKSIGVERIFY)
                .push_x_only_key(&attestor_xonlypubkey)
                .push_opcode(all::OP_CHECKSIG)
                .into_script(),
        };

        let builder =
            TaprootBuilder::with_huffman_tree(vec![(1, multisig_script.clone())]).unwrap_throw();

        let tap_tree = TapTree::try_from(builder).unwrap_throw();

        let tap_internal_unspendable_key =
            XOnlyPublicKey::from_str(TAPROOT_UNSPENDABLE_KEY).unwrap_throw();

        let tap_info = match tap_tree
            .into_builder()
            .finalize(&secp, tap_internal_unspendable_key)
        {
            Ok(tap_info) => tap_info,
            Err(e) => {
                clog!("Error finalizing taproot builder: {:?}", e);
                return Err(JsError::new("Error finalizing taproot builder"));
            }
        };

        let tx_out = closing_psbt.inputs[0].witness_utxo.clone().unwrap_throw();

        let merkle_root = tap_info.merkle_root();

        let tweak =
            TapTweakHash::from_key_and_tweak(tap_internal_unspendable_key, merkle_root).to_scalar();
        let tweaked_pubkey = tap_internal_unspendable_key
            .add_tweak(&secp, &tweak)
            .map_err(|e| JsError::new(&e.to_string()))?
            .0;

        let sighash_sig = SighashCache::new(&closing_psbt.unsigned_tx.clone())
            .taproot_script_spend_signature_hash(
                0,
                &Prevouts::All(&[tx_out.clone()]),
                ScriptPath::with_defaults(&multisig_script),
                SchnorrSighashType::Default,
            )
            .unwrap_throw();

        let attestor_sig = secp.sign_schnorr(
            &Message::from_slice(&sighash_sig).unwrap_throw(),
            &attestor_keypair,
        );

        let actual_control = tap_info
            .control_block(&(multisig_script.clone(), LeafVersion::TapScript))
            .expect_throw("woopsie daisy");

        let verification =
            actual_control.verify_taproot_commitment(&secp, tweaked_pubkey, &multisig_script);

        // if verification fails, we should return an error
        if !verification {
            return Err(JsError::new("Error verifying taproot commitment"));
        }

        let mut input = Input::default();

        let mut b_tree_map = BTreeMap::<ControlBlock, (Script, LeafVersion)>::default();
        b_tree_map.insert(
            actual_control.clone(),
            (multisig_script.clone(), LeafVersion::TapScript),
        );

        input.tap_scripts = b_tree_map;
        input.tap_internal_key = Some(tap_info.internal_key());

        input.witness_utxo = Some(tx_out);
        input.tap_merkle_root = tap_info.merkle_root();

        let mut pst = PartiallySignedTransaction {
            unsigned_tx: closing_psbt.unsigned_tx.clone(),
            version: 2,
            xpub: BTreeMap::default(),
            proprietary: BTreeMap::default(),
            unknown: BTreeMap::default(),
            inputs: vec![input],
            outputs: vec![],
        };

        let attestor_schnorr_sig = SchnorrSig {
            sig: attestor_sig,
            hash_ty: SchnorrSighashType::Default,
        };

        let alice_schnorr_sig = closing_psbt.inputs[0]
            .tap_script_sigs
            .first_key_value()
            .unwrap_throw()
            .1;

        match is_attestor_key_first_in_multisig_script_order {
            true => {
                pst.inputs[0].final_script_witness = Some(Witness::from_vec(vec![
                    alice_schnorr_sig.to_vec(),
                    attestor_schnorr_sig.to_vec(),
                    multisig_script.to_bytes(),
                    actual_control.serialize(),
                ]));
            }
            false => {
                pst.inputs[0].final_script_witness = Some(Witness::from_vec(vec![
                    attestor_schnorr_sig.to_vec(),
                    alice_schnorr_sig.to_vec(),
                    multisig_script.to_bytes(),
                    actual_control.serialize(),
                ]));
            }
        }

        let closing_tx = closing_psbt.clone().extract_tx();

        let pst_tx = pst.extract_tx();

        // post the pst_tx bitcoin transaction to esplora api using reqwest
        let client = reqwest::Client::new();
        let res = client
            .post("https://devnet.dlc.link/electrs/tx")
            .body(bitcoin::consensus::encode::serialize_hex(&pst_tx))
            .send()
            .await
            .map_err(|e| JsError::new(&format!("Error posting tx to esplora: {}", e)))?;

        let status = res.status();
        let message = res.text().await.unwrap_throw();
        const ALREADY_BROADCAST_STRING: &str = "sendrawtransaction RPC error: {\"code\":-27,\"message\":\"Transaction already in block chain\"}";
        clog!("status: {}, message: {}", status, message);
        match (status, &message) {
            (reqwest::StatusCode::OK, _) => {
                clog!(
                    "Broadcasting closing tx to esplora successful!, txid: {}",
                    pst_tx.txid()
                );
            }
            (reqwest::StatusCode::BAD_REQUEST, message) if message == ALREADY_BROADCAST_STRING => {
                clog!(
                    "Closing tx already in blockchain, this is not an error, txid: {}",
                    pst_tx.txid()
                );
            }
            (_, _) => {
                return Err(JsError::new(&format!(
                    "Error posting tx to esplora: {}, {}",
                    status, message
                )));
            }
        }

        let closing_tx_value = serde_wasm_bindgen::to_value(&closing_tx)
            .map_err(|_| JsError::new("Error serializing closing_tx to JSON"))?;

        // TODO: Status:CLosing?
        // Set the status of the psbt event to closed
        let update_psbt_db_value = PsbtDbValue(
            bitcoin::consensus::encode::serialize(&psbt_event.closing_psbt).to_hex(),
            psbt_event.mint_address.to_string(),
            uuid.to_string(),
            psbt_event.funding_txid,
            None,                    //outcome
            PsbtEventStatus::Closed, //status
            psbt_event.chain,        //chain name
        );

        let new_psbt_event = serde_json::to_string(&update_psbt_db_value)
            .map_err(|_| JsError::new("Error serializing new_event to JSON"))?
            .into_bytes();

        match &self
            .oracle
            .event_handler
            .storage_api
            .clone()
            .insert(uuid.to_string(), new_psbt_event.clone(), self.secret_key)
            .await
        {
            Ok(Some(_val)) => (),
            _ => {
                clog!(
                "[WASM-ATTESTOR] Unable to update psbt event in StorageAPI with uuid: {}, failed to create psbt locking event",
                uuid
            );
                return Err(JsError::new("Failed to update psbt closing event"));
            }
        }

        Ok(pst_tx.txid())
    }

    async fn set_psbt_event_status(
        &self,
        uuid: &str,
        to_status: PsbtEventStatus,
    ) -> Result<(), JsError> {
        let psbt_event = self.get_psbt_event(uuid.to_string()).await?;
        let update_psbt_db_value = PsbtDbValue(
            bitcoin::consensus::encode::serialize(&psbt_event.closing_psbt).to_hex(),
            psbt_event.mint_address.to_string(),
            uuid.to_string(),
            psbt_event.funding_txid,
            None,              //outcome
            to_status.clone(), //status
            psbt_event.chain,  //chain name
        );

        let new_psbt_event = serde_json::to_string(&update_psbt_db_value)
            .map_err(|_| JsError::new("Error serializing new_event to JSON"))?
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
                    "[WASM-ATTESTOR] Unable to update psbt event in StorageAPI with uuid: {} to status {}",
                    uuid, to_status
                );
                Err(JsError::new(&format!(
                    "Failed to update psbt event status to {to_status}"
                )))
            }
        }
    }

    /// callback for setting the db status to funded for PSBT events
    pub async fn set_psbt_event_to_funded(&self, uuid: &str) -> Result<(), JsError> {
        self.set_psbt_event_status(uuid, PsbtEventStatus::Funded)
            .await
    }

    pub async fn set_psbt_event_to_funded(&self, uuid: &str) -> Result<(), JsError> {
        self.set_psbt_event_status(uuid, PsbtEventStatus::Funded)
            .await
    }

    /// Iterates through psbt events that are in the pending or confirmed states
    /// if it and have over 6 confirmations and are funded, they are set to confirmed
    /// then all confirmed events are returned
    pub async fn get_confirmed_psbt_events(&self) -> Result<JsValue, JsError> {
        let psbt_events = self.get_all_psbt_events().await?;

        let pending_confirmed_events: Vec<ApiOraclePsbtEvent> = psbt_events
            .into_iter()
            .filter(|event| {
                [PsbtEventStatus::Pending, PsbtEventStatus::Confirmed].contains(&event.status)
            })
            .collect();

        // TODO: change this to an async filter, like in wallet/src/main.rs:126
        let mut to_confirm_events: Vec<ApiOraclePsbtEvent> = Vec::new();
        for event in pending_confirmed_events {
            let confirmed = self
                .get_validation_status_for_uuid(event.uuid.clone())
                .await?;
            if confirmed {
                to_confirm_events.push(event.clone());
                if event.status == PsbtEventStatus::Pending {
                    self.set_psbt_event_status(&event.uuid, PsbtEventStatus::Confirmed)
                        .await?;
                }
            }
        }

        serde_wasm_bindgen::to_value(&to_confirm_events)
            .map_err(|_| JsError::new("[WASM-ATTESTOR] Error serializing psbt events to JSON"))
    }

    /// This function is used to validate the funding tx
    /// It will return a boolean value indicating if the funding tx is valid
    pub async fn get_validation_status_for_uuid(&self, uuid: String) -> Result<bool, JsError> {
        // recreate the transactions, and make sure they match, minus the signatures
        // make sure the internal tap key is invalid.

        // Use the bdk verify functions to verify the sigs on the PSBTs

        let psbt_event = self.get_psbt_event(uuid.clone()).await?;
        let funding_txid = psbt_event.funding_txid;

        let blockchain = EsploraBlockchain::new("https://devnet.dlc.link/electrs", 20);
        let tx_status = blockchain
            .get_tx_status(&bitcoin::Txid::from_str(&funding_txid)?)
            .await?
            .ok_or(JsError::new(&format!(
                "Error getting tx status, maybe tx doesnt exist tx {funding_txid}"
            )))?;

        let block_chain_height = blockchain
            .get_height()
            .await
            .map_err(|e| JsError::new(&format!("Unable to get chain height {e}")))?
            as u64;

        let confirmations = match (tx_status.confirmed, tx_status.block_height) {
            (true, Some(block_height)) => (block_chain_height - block_height as u64 + 1) as u32,
            _ => 0,
        };

        Ok(confirmations >= 6)
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub enum PsbtEventStatus {
    Pending,
    Confirmed,
    Funded,
    Closed,
}

// implement display for PsbtEventStatus
impl std::fmt::Display for PsbtEventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PsbtEventStatus::Pending => write!(f, "Pending"),
            PsbtEventStatus::Confirmed => write!(f, "Confirmed"),
            PsbtEventStatus::Funded => write!(f, "Funded"),
            PsbtEventStatus::Closed => write!(f, "Closed"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
struct ApiOraclePsbtEvent {
    event_id: String,
    uuid: String,
    funding_txid: String,
    closing_psbt: PartiallySignedTransaction,
    mint_address: String,
    outcome: Option<u64>,
    status: PsbtEventStatus,
    chain: Option<String>,
}

fn parse_psbt_database_entry(
    event_binary: Vec<u8>,
) -> Result<ApiOraclePsbtEvent, GenericOracleError> {
    let event: PsbtDbValue =
        serde_json::from_str(&String::from_utf8(event_binary.clone()).expect("to string"))
            .map_err(|_| GenericOracleError {
                message: "Error deserializing new_event from JSON".to_string(),
            })?;

    let closing_psbt = event.0.clone();

    let closing_psbt: Vec<u8> =
        FromHex::from_hex(&closing_psbt).expect("to decode funding_psbt hex");
    let closing_psbt: PartiallySignedTransaction =
        deserialize(&closing_psbt).map_err(|_| GenericOracleError {
            message: "Unable to deserialize closing psbt event from db".to_string(),
        })?;

    Ok(ApiOraclePsbtEvent {
        event_id: event.2.clone(),
        uuid: event.2,
        closing_psbt,
        funding_txid: event.3,
        mint_address: event.1,
        outcome: event.4,
        status: event.5,
        chain: event.6,
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

fn parse_database_entry(event: Vec<u8>) -> Result<ApiOracleEvent, JsError> {
    let event_str = String::from_utf8_lossy(&event);
    let event: DbValue = serde_json::from_str(&event_str)
        .map_err(|_| JsError::new("[WASM-ATTESTOR] Error parsing event from string"))?;

    let announcement_vec = event.1.clone();
    let mut cursor = Cursor::new(&announcement_vec);
    let announcement = OracleAnnouncement::read(&mut cursor)
        .map_err(|_| JsError::new("[WASM-ATTESTOR] Error reading OracleAnnouncement"))?;

    let db_att = event.2.clone();
    let decoded_att_json = match db_att {
        None => None,
        Some(att_vec) => {
            let mut attestation_cursor = Cursor::new(&att_vec);

            match OracleAttestation::read(&mut attestation_cursor) {
                Ok(att) => Some(format!("{:?}", att)),
                Err(_) => return Err(JsError::new("[WASM-ATTESTOR] Error decoding attestation")),
            }
        }
    };

    let rust_announcement_json = serde_json::to_string(&announcement)
        .map_err(|_| JsError::new("[WASM-ATTESTOR] Error serializing announcement to JSON"))?;

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
