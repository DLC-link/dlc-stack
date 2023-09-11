#![feature(async_fn_in_trait)]
//! #Manager a component to create and update DLCs.

extern crate dlc_manager;

use crate::dlc_manager::contract::{
    accepted_contract::AcceptedContract, contract_info::ContractInfo,
    contract_input::ContractInput, offered_contract::OfferedContract,
    signed_contract::SignedContract, AdaptorInfo, ClosedContract, Contract, FailedAcceptContract,
    FailedSignContract, PreClosedContract,
};
use crate::dlc_manager::contract_updater::{accept_contract, verify_accepted_and_sign_contract};
use crate::dlc_manager::error::Error;
use crate::dlc_manager::{Blockchain, Time, Wallet};

use bitcoin::Address;
use bitcoin::Transaction;

use dlc_manager::ContractId;
use dlc_messages::oracle_msgs::{OracleAnnouncement, OracleAttestation};
use dlc_messages::{AcceptDlc, Message as DlcMessage, OfferDlc, SignDlc};

use log::*;
use secp256k1_zkp::XOnlyPublicKey;
use secp256k1_zkp::{All, PublicKey, Secp256k1};
use std::collections::HashMap;
use std::ops::Deref;
use std::string::ToString;

/// The number of confirmations required before moving the the confirmed state.
pub const NB_CONFIRMATIONS: u32 = 6;
/// The delay to set the refund value to.
pub const REFUND_DELAY: u32 = 86400 * 7;
/// The nSequence value used for CETs in DLC channels
pub const CET_NSEQUENCE: u32 = 288;
/// Timeout in seconds when waiting for a peer's reply, after which a DLC channel
/// is forced closed.
pub const PEER_TIMEOUT: u64 = 3600;

type ClosableContractInfo<'a> = Option<(
    &'a ContractInfo,
    &'a AdaptorInfo,
    Vec<(usize, OracleAttestation)>,
)>;

/// Oracle trait provides access to oracle information.
pub trait AsyncOracle {
    /// Returns the public key of the oracle.
    async fn get_public_key(&self) -> XOnlyPublicKey;
    /// Returns the announcement for the event with the given id if found.
    async fn get_announcement(&self, event_id: &str) -> Result<OracleAnnouncement, Error>;
    /// Returns the attestation for the event with the given id if found.
    async fn get_attestation(&self, event_id: &str) -> Result<OracleAttestation, Error>;
}

pub trait AsyncStorage {
    /// Returns the contract with given id if found.
    async fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, Error>;
    /// Return all contracts
    async fn get_contracts(&self) -> Result<Vec<Contract>, Error>;
    /// Create a record for the given contract.
    async fn create_contract(&self, contract: &OfferedContract) -> Result<(), Error>;
    /// Delete the record for the contract with the given id.
    async fn delete_contract(&self, id: &ContractId) -> Result<(), Error>;
    /// Update the given contract.
    async fn update_contract(&self, contract: &Contract) -> Result<(), Error>;
    /// Returns the set of contracts in offered state.
    async fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, Error>;
    /// Returns the set of contracts in signed state.
    async fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, Error>;
    /// Returns the set of confirmed contracts.
    async fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, Error>;
    /// Returns the set of contracts whos broadcasted cet has not been verified to be confirmed on
    /// blockchain
    async fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error>;
}

/// Used to create and update DLCs.
pub struct Manager<W: Deref, B: Deref, S: Deref, O: Deref, T: Deref>
where
    W::Target: Wallet,
    B::Target: Blockchain,
    S::Target: AsyncStorage,
    O::Target: AsyncOracle,
    T::Target: Time,
{
    oracles: HashMap<XOnlyPublicKey, O>,
    wallet: W,
    blockchain: B,
    store: S,
    secp: Secp256k1<All>,
    time: T,
}

macro_rules! get_object_in_state {
    ($manager: ident, $id: expr, $state: ident, $peer_id: expr, $object_type: ident, $get_call: ident) => {{
        let object = $manager.store.$get_call($id).await?;
        match object {
            Some(c) => {
                if let Some(p) = $peer_id as Option<PublicKey> {
                    if c.get_counter_party_id() != p {
                        return Err(Error::InvalidParameters(format!(
                            "Peer {:02x?} is not involved with contract {:02x?}.",
                            $peer_id, $id
                        )));
                    }
                }
                match c {
                    $object_type::$state(s) => Ok(s),
                    _ => Err(Error::InvalidState(format!(
                        "Invalid state {:?} expected {}.",
                        c,
                        stringify!($state),
                    ))),
                }
            }
            None => Err(Error::InvalidParameters(format!(
                "Unknown {} id.",
                stringify!($object_type)
            ))),
        }
    }};
}

macro_rules! get_contract_in_state {
    ($manager: ident, $contract_id: expr, $state: ident, $peer_id: expr) => {{
        get_object_in_state!(
            $manager,
            $contract_id,
            $state,
            $peer_id,
            Contract,
            get_contract
        )
    }};
}

impl<W: Deref, B: Deref, S: Deref, O: Deref, T: Deref> Manager<W, B, S, O, T>
where
    W::Target: Wallet,
    B::Target: Blockchain,
    S::Target: AsyncStorage,
    O::Target: AsyncOracle,
    T::Target: Time,
{
    /// Create a new Manager struct.
    pub fn new(
        wallet: W,
        blockchain: B,
        store: S,
        oracles: HashMap<XOnlyPublicKey, O>,
        time: T,
    ) -> Result<Self, Error> {
        Ok(Manager {
            secp: secp256k1_zkp::Secp256k1::new(),
            wallet,
            blockchain,
            store,
            oracles,
            time,
        })
    }

    /// Get the store from the Manager to access contracts.
    pub fn get_store(&self) -> &S {
        &self.store
    }

    #[doc(hidden)]
    pub fn get_mut_store(&mut self) -> &mut S {
        &mut self.store
    }

    /// Function called to pass a DlcMessage to the Manager.
    pub async fn on_dlc_message(
        &mut self,
        msg: &DlcMessage,
        counter_party: PublicKey,
    ) -> Result<Option<DlcMessage>, Error> {
        match msg {
            DlcMessage::Offer(o) => {
                self.on_offer_message(o, counter_party).await?;
                Ok(None)
            }
            DlcMessage::Accept(a) => Ok(Some(self.on_accept_message(a, &counter_party).await?)),
            DlcMessage::Sign(s) => {
                self.on_sign_message(s, &counter_party).await?;
                Ok(None)
            }
            _ => Err(Error::InvalidState("Invalid message type.".to_string())),
        }
    }

    /// Function called to create a new DLC. The offered contract will be stored
    /// and an OfferDlc message returned.
    pub async fn send_offer(
        &mut self,
        contract_input: &ContractInput,
        counter_party: PublicKey,
    ) -> Result<OfferDlc, Error> {
        contract_input.validate()?;

        // let contract_infos = &contract_input.contract_infos;
        // let mut oracle_announcements = Vec::new();
        let event_id = &contract_input
            .contract_infos
            .first()
            .unwrap()
            .oracles
            .event_id;
        let oracle_set: Vec<Vec<&O>> = contract_input
            .contract_infos
            .iter()
            .map(|x| {
                x.oracles
                    .public_keys
                    .iter()
                    .map(|pubkey| {
                        self.oracles
                            .get(pubkey)
                            .ok_or_else(|| {
                                Error::InvalidParameters("Unknown oracle public key".to_string())
                            })
                            .unwrap()
                    })
                    .collect::<Vec<&O>>()
            })
            .collect::<Vec<_>>();

        let mut oracle_announcements = Vec::new();

        for oracles in oracle_set {
            let mut announcements = Vec::new();
            for oracle in oracles {
                announcements.push(oracle.get_announcement(event_id).await?);
            }
            oracle_announcements.push(announcements)
        }

        let (offered_contract, offer_msg) = crate::dlc_manager::contract_updater::offer_contract(
            &self.secp,
            contract_input,
            oracle_announcements,
            REFUND_DELAY,
            &counter_party,
            &self.wallet,
            &self.blockchain,
            &self.time,
        )?;

        offered_contract.validate()?;

        self.store.create_contract(&offered_contract).await?;

        Ok(offer_msg)
    }

    /// Function to call to accept a DLC for which an offer was received.
    pub async fn accept_contract_offer(
        &mut self,
        contract_id: &ContractId,
    ) -> Result<(ContractId, PublicKey, AcceptDlc), Error> {
        let offered_contract =
            get_contract_in_state!(self, contract_id, Offered, None as Option<PublicKey>)?;

        let counter_party = offered_contract.counter_party;

        let (accepted_contract, accept_msg) = accept_contract(
            &self.secp,
            &offered_contract,
            &self.wallet,
            &self.blockchain,
        )?;

        self.wallet.import_address(&Address::p2wsh(
            &accepted_contract.dlc_transactions.funding_script_pubkey,
            self.blockchain.get_network()?,
        ))?;

        let contract_id = accepted_contract.get_contract_id();

        self.store
            .update_contract(&Contract::Accepted(accepted_contract))
            .await?;

        Ok((contract_id, counter_party, accept_msg))
    }

    /// Function to call to check the state of the currently executing DLCs and
    /// update them if possible.
    pub async fn periodic_check(&mut self) -> Result<Vec<(ContractId, String)>, Error> {
        let mut affected_contracts = Vec::<(ContractId, String)>::new();
        affected_contracts.extend_from_slice(&self.check_signed_contracts().await?);
        affected_contracts.extend_from_slice(&self.check_confirmed_contracts().await?);
        affected_contracts.extend_from_slice(&self.check_preclosed_contracts().await?);

        Ok(affected_contracts)
    }

    async fn on_offer_message(
        &mut self,
        offered_message: &OfferDlc,
        counter_party: PublicKey,
    ) -> Result<(), Error> {
        offered_message.validate(&self.secp, REFUND_DELAY, REFUND_DELAY * 2)?;
        let contract: OfferedContract =
            OfferedContract::try_from_offer_dlc(offered_message, counter_party)?;
        contract.validate()?;

        if self.store.get_contract(&contract.id).await?.is_some() {
            return Err(Error::InvalidParameters(
                "Contract with identical id already exists".to_string(),
            ));
        }

        self.store.create_contract(&contract).await?;

        Ok(())
    }

    async fn on_accept_message(
        &mut self,
        accept_msg: &AcceptDlc,
        counter_party: &PublicKey,
    ) -> Result<DlcMessage, Error> {
        let offered_contract = get_contract_in_state!(
            self,
            &accept_msg.temporary_contract_id,
            Offered,
            Some(*counter_party)
        )?;

        let (signed_contract, signed_msg) = match verify_accepted_and_sign_contract(
            &self.secp,
            &offered_contract,
            accept_msg,
            &self.wallet,
        ) {
            Ok(contract) => contract,
            Err(e) => {
                return self
                    .accept_fail_on_error(offered_contract, accept_msg.clone(), e)
                    .await
            }
        };

        self.wallet.import_address(&Address::p2wsh(
            &signed_contract
                .accepted_contract
                .dlc_transactions
                .funding_script_pubkey,
            self.blockchain.get_network()?,
        ))?;

        self.store
            .update_contract(&Contract::Signed(signed_contract))
            .await?;

        Ok(DlcMessage::Sign(signed_msg))
    }

    async fn on_sign_message(
        &mut self,
        sign_message: &SignDlc,
        peer_id: &PublicKey,
    ) -> Result<(), Error> {
        let accepted_contract =
            get_contract_in_state!(self, &sign_message.contract_id, Accepted, Some(*peer_id))?;

        let (signed_contract, fund_tx) =
            match crate::dlc_manager::contract_updater::verify_signed_contract(
                &self.secp,
                &accepted_contract,
                sign_message,
                &self.wallet,
            ) {
                Ok(contract) => contract,
                Err(e) => {
                    return self
                        .sign_fail_on_error(accepted_contract, sign_message.clone(), e)
                        .await
                }
            };

        self.store
            .update_contract(&Contract::Signed(signed_contract))
            .await?;

        self.blockchain.send_transaction(&fund_tx)?;

        Ok(())
    }

    async fn sign_fail_on_error<R>(
        &mut self,
        accepted_contract: AcceptedContract,
        sign_message: SignDlc,
        e: Error,
    ) -> Result<R, Error> {
        error!("Error in on_sign {}", e);
        self.store
            .update_contract(&Contract::FailedSign(FailedSignContract {
                accepted_contract,
                sign_message,
                error_message: e.to_string(),
            }))
            .await?;
        Err(e)
    }

    async fn accept_fail_on_error<R>(
        &mut self,
        offered_contract: OfferedContract,
        accept_message: AcceptDlc,
        e: Error,
    ) -> Result<R, Error> {
        error!("Error in on_accept {}", e);
        self.store
            .update_contract(&Contract::FailedAccept(FailedAcceptContract {
                offered_contract,
                accept_message,
                error_message: e.to_string(),
            }))
            .await?;
        Err(e)
    }

    async fn check_signed_contract(&mut self, contract: &SignedContract) -> Result<bool, Error> {
        let confirmations = self.blockchain.get_transaction_confirmations(
            &contract.accepted_contract.dlc_transactions.fund.txid(),
        )?;
        if confirmations >= NB_CONFIRMATIONS {
            self.store
                .update_contract(&Contract::Confirmed(contract.clone()))
                .await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn check_signed_contracts(&mut self) -> Result<Vec<(ContractId, String)>, Error> {
        let mut contracts_to_confirm = Vec::new();
        for c in self.store.get_signed_contracts().await? {
            match self.check_signed_contract(&c).await {
                Ok(true) => contracts_to_confirm.push((
                    c.accepted_contract.get_contract_id(),
                    c.accepted_contract
                        .offered_contract
                        .contract_info
                        .get(0)
                        .unwrap()
                        .oracle_announcements
                        .get(0)
                        .unwrap()
                        .oracle_event
                        .event_id
                        .clone(),
                )),
                Ok(false) => (),
                Err(e) => error!(
                    "Error checking confirmed contract {}: {}",
                    c.accepted_contract.get_contract_id_string(),
                    e
                ),
            }
        }

        Ok(contracts_to_confirm)
    }

    async fn check_confirmed_contracts(&mut self) -> Result<Vec<(ContractId, String)>, Error> {
        let mut contracts_to_close = Vec::new();
        for c in self.store.get_confirmed_contracts().await? {
            // Confirmed contracts from channel are processed in channel specific methods.
            if c.channel_id.is_some() {
                continue;
            }
            match self.check_confirmed_contract(&c).await {
                Err(e) => {
                    error!(
                        "Error checking confirmed contract {}: {}",
                        c.accepted_contract.get_contract_id_string(),
                        e
                    )
                }
                Ok(true) => contracts_to_close.push((
                    c.accepted_contract.get_contract_id(),
                    c.accepted_contract
                        .offered_contract
                        .contract_info
                        .get(0)
                        .unwrap()
                        .oracle_announcements
                        .get(0)
                        .unwrap()
                        .oracle_event
                        .event_id
                        .clone(),
                )),
                Ok(false) => (),
            }
        }

        Ok(contracts_to_close)
    }

    async fn get_closable_contract_info<'a>(
        &'a self,
        contract: &'a SignedContract,
    ) -> ClosableContractInfo<'a> {
        let contract_infos = &contract.accepted_contract.offered_contract.contract_info;
        let adaptor_infos = &contract.accepted_contract.adaptor_infos;
        for (contract_info, adaptor_info) in contract_infos.iter().zip(adaptor_infos.iter()) {
            let announcements: Vec<(usize, &OracleAnnouncement)> = contract_info
                .oracle_announcements
                .iter()
                .enumerate()
                .collect();
            if announcements.len() >= contract_info.threshold {
                let attestations: Vec<_> = futures::future::join_all(
                    announcements.iter().filter_map(|(i, announcement)| {
                        Some(async move {
                            let oracle = self.oracles.get(&announcement.oracle_public_key).unwrap();
                            (
                                *i,
                                oracle
                                    .get_attestation(&announcement.oracle_event.event_id)
                                    .await
                                    .unwrap(), // .ok(),
                            )
                        })
                    }),
                )
                .await;
                if attestations.len() >= contract_info.threshold {
                    return Some((contract_info, adaptor_info, attestations));
                }
            }
        }
        None
    }

    async fn check_confirmed_contract(&mut self, contract: &SignedContract) -> Result<bool, Error> {
        let closable_contract_info = self.get_closable_contract_info(contract).await;
        if let Some((contract_info, adaptor_info, attestations)) = closable_contract_info {
            let cet = crate::dlc_manager::contract_updater::get_signed_cet(
                &self.secp,
                contract,
                contract_info,
                adaptor_info,
                &attestations,
                &self.wallet,
            )?;
            match self.close_contract(
                contract,
                cet,
                attestations.iter().map(|x| x.1.clone()).collect(),
            ) {
                Ok(closed_contract) => {
                    self.store.update_contract(&closed_contract).await?;
                    return Ok(true);
                }
                Err(e) => {
                    warn!(
                        "Failed to close contract {}: {}",
                        contract.accepted_contract.get_contract_id_string(),
                        e
                    );
                    return Err(e);
                }
            }
        }
        self.check_refund(contract).await?;

        Ok(false)
    }

    async fn check_preclosed_contracts(&mut self) -> Result<Vec<(ContractId, String)>, Error> {
        let mut contracts_to_close = Vec::new();
        for c in self.store.get_preclosed_contracts().await? {
            match self.check_preclosed_contract(&c).await {
                Ok(true) => contracts_to_close.push((
                    c.signed_contract.accepted_contract.get_contract_id(),
                    c.signed_contract
                        .accepted_contract
                        .offered_contract
                        .contract_info
                        .get(0)
                        .unwrap()
                        .oracle_announcements
                        .get(0)
                        .unwrap()
                        .oracle_event
                        .event_id
                        .clone(),
                )),
                Ok(false) => (),
                Err(e) => error!(
                    "Error checking pre-closed contract {}: {}",
                    c.signed_contract.accepted_contract.get_contract_id_string(),
                    e
                ),
            }
        }

        Ok(contracts_to_close)
    }

    async fn check_preclosed_contract(
        &mut self,
        contract: &PreClosedContract,
    ) -> Result<bool, Error> {
        let broadcasted_txid = contract.signed_cet.txid();
        let confirmations = self
            .blockchain
            .get_transaction_confirmations(&broadcasted_txid)?;
        if confirmations >= NB_CONFIRMATIONS {
            let closed_contract = ClosedContract {
                attestations: contract.attestations.clone(),
                signed_cet: Some(contract.signed_cet.clone()),
                contract_id: contract.signed_contract.accepted_contract.get_contract_id(),
                temporary_contract_id: contract
                    .signed_contract
                    .accepted_contract
                    .offered_contract
                    .id,
                counter_party_id: contract
                    .signed_contract
                    .accepted_contract
                    .offered_contract
                    .counter_party,
                pnl: contract
                    .signed_contract
                    .accepted_contract
                    .compute_pnl(&contract.signed_cet),
            };
            self.store
                .update_contract(&Contract::Closed(closed_contract.clone()))
                .await?;
            return Ok(true);
        }

        Ok(false)
    }

    fn close_contract(
        &mut self,
        contract: &SignedContract,
        signed_cet: Transaction,
        attestations: Vec<OracleAttestation>,
    ) -> Result<Contract, Error> {
        let confirmations = self
            .blockchain
            .get_transaction_confirmations(&signed_cet.txid())?;

        // Put it here for post-close, and here's the btc txid too.
        // But perhaps we'd rather have it in the final close place, and
        // only set it after 6 confirmations

        if confirmations < 1 {
            // TODO(tibo): if this fails because another tx is already in
            // mempool or blockchain, we might have been cheated. There is
            // not much to be done apart from possibly extracting a fraud
            // proof but ideally it should be handled.
            self.blockchain.send_transaction(&signed_cet)?;

            let preclosed_contract = PreClosedContract {
                signed_contract: contract.clone(),
                attestations: Some(attestations),
                signed_cet,
            };

            return Ok(Contract::PreClosed(preclosed_contract));
        } else if confirmations < NB_CONFIRMATIONS {
            let preclosed_contract = PreClosedContract {
                signed_contract: contract.clone(),
                attestations: Some(attestations),
                signed_cet,
            };

            return Ok(Contract::PreClosed(preclosed_contract));
        }

        let closed_contract = ClosedContract {
            attestations: Some(attestations.to_vec()),
            pnl: contract.accepted_contract.compute_pnl(&signed_cet),
            signed_cet: Some(signed_cet),
            contract_id: contract.accepted_contract.get_contract_id(),
            temporary_contract_id: contract.accepted_contract.offered_contract.id,
            counter_party_id: contract.accepted_contract.offered_contract.counter_party,
        };

        Ok(Contract::Closed(closed_contract))
    }

    async fn check_refund(&mut self, contract: &SignedContract) -> Result<(), Error> {
        // TODO(tibo): should check for confirmation of refund before updating state
        if contract
            .accepted_contract
            .dlc_transactions
            .refund
            .lock_time
            .0 as u64
            <= self.time.unix_time_now()
        {
            let accepted_contract = &contract.accepted_contract;
            let refund = accepted_contract.dlc_transactions.refund.clone();
            let confirmations = self
                .blockchain
                .get_transaction_confirmations(&refund.txid())?;
            if confirmations == 0 {
                let refund = crate::dlc_manager::contract_updater::get_signed_refund(
                    &self.secp,
                    contract,
                    &self.wallet,
                )?;
                self.blockchain.send_transaction(&refund)?;
            }

            self.store
                .update_contract(&Contract::Refunded(contract.clone()))
                .await?;
        }

        Ok(())
    }
}

// #[cfg(test)]
// mod test {
//     use dlc_messages::Message;
//     use mocks::{
//         dlc_manager::{manager::Manager, Oracle},
//         memory_storage_provider::MemoryStorage,
//         mock_blockchain::MockBlockchain,
//         mock_oracle_provider::MockOracle,
//         mock_time::MockTime,
//         mock_wallet::MockWallet,
//     };
//     use secp256k1_zkp::PublicKey;
//     use std::{collections::HashMap, rc::Rc};

//     type TestManager = Manager<
//         Rc<MockWallet>,
//         Rc<MockBlockchain>,
//         Rc<MemoryStorage>,
//         Rc<MockOracle>,
//         Rc<MockTime>,
//         Rc<MockBlockchain>,
//     >;

//     fn get_manager() -> TestManager {
//         let blockchain = Rc::new(MockBlockchain::new());
//         let store = Rc::new(MemoryStorage::new());
//         let wallet = Rc::new(MockWallet::new(
//             &blockchain,
//             &(0..100).map(|x| x as u64 * 1000000).collect::<Vec<_>>(),
//         ));

//         let oracle_list = (0..5).map(|_| MockOracle::new()).collect::<Vec<_>>();
//         let oracles: HashMap<bitcoin::XOnlyPublicKey, _> = oracle_list
//             .into_iter()
//             .map(|x| (x.get_public_key(), Rc::new(x)))
//             .collect();
//         let time = Rc::new(MockTime {});

//         mocks::mock_time::set_time(0);

//         Manager::new(wallet, blockchain.clone(), store, oracles, time, blockchain).unwrap()
//     }

//     fn pubkey() -> PublicKey {
//         "0218845781f631c48f1c9709e23092067d06837f30aa0cd0544ac887fe91ddd166"
//             .parse()
//             .unwrap()
//     }

//     #[test]
//     fn reject_offer_with_existing_contract_id() {
//         let offer_message = Message::Offer(
//             serde_json::from_str(include_str!("../test_inputs/offer_contract.json")).unwrap(),
//         );

//         let mut manager = get_manager();

//         manager
//             .on_dlc_message(&offer_message, pubkey())
//             .expect("To accept the first offer message");

//         manager
//             .on_dlc_message(&offer_message, pubkey())
//             .expect_err("To reject the second offer message");
//     }

//     #[test]
//     fn reject_channel_offer_with_existing_channel_id() {
//         let offer_message = Message::OfferChannel(
//             serde_json::from_str(include_str!("../test_inputs/offer_channel.json")).unwrap(),
//         );

//         let mut manager = get_manager();

//         manager
//             .on_dlc_message(&offer_message, pubkey())
//             .expect("To accept the first offer message");

//         manager
//             .on_dlc_message(&offer_message, pubkey())
//             .expect_err("To reject the second offer message");
//     }
// }
