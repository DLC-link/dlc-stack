#![feature(async_fn_in_trait)]

use bitcoin::{Address, OutPoint, Txid};
use dlc_manager::chain_monitor::ChainMonitor;
use dlc_manager::channel::{
    offered_channel::OfferedChannel,
    signed_channel::{SignedChannel, SignedChannelStateType},
    Channel,
};
use dlc_manager::contract::{
    offered_contract::OfferedContract, signed_contract::SignedContract, Contract, PreClosedContract,
};

use dlc_link_manager::AsyncStorage;
use dlc_manager::{error::Error as DaemonError, ChannelId, ContractId, Utxo};
use secp256k1_zkp::{PublicKey, SecretKey};
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

pub struct DlcMemoryStorageProvider {
    contracts: RwLock<HashMap<ContractId, Contract>>,
    // channels: RwLock<HashMap<ChannelId, Channel>>,
    contracts_saved: Mutex<Option<HashMap<ContractId, Contract>>>,
    // channels_saved: Mutex<Option<HashMap<ChannelId, Channel>>>,
    // addresses: RwLock<HashMap<Address, SecretKey>>,
    // utxos: RwLock<HashMap<OutPoint, Utxo>>,
    // key_pairs: RwLock<HashMap<PublicKey, SecretKey>>,
}

impl DlcMemoryStorageProvider {
    pub fn new() -> Self {
        DlcMemoryStorageProvider {
            contracts: RwLock::new(HashMap::new()),
            // channels: RwLock::new(HashMap::new()),
            contracts_saved: Mutex::new(None),
            // channels_saved: Mutex::new(None),
            // addresses: RwLock::new(HashMap::new()),
            // utxos: RwLock::new(HashMap::new()),
            // key_pairs: RwLock::new(HashMap::new()),
        }
    }

    pub fn save(&self) {
        let mut contracts_saved = self.contracts_saved.lock().unwrap();

        *contracts_saved = Some(
            self.contracts
                .read()
                .expect("Could not get read lock")
                .clone(),
        );
        // let mut channels_saved = self.channels_saved.lock().unwrap();
        // *channels_saved = Some(
        //     self.channels
        //         .read()
        //         .expect("Could not get read lock")
        //         .clone(),
        // );
    }

    pub fn rollback(&self) {
        let mut contracts = self.contracts.write().unwrap();
        let mut contracts_saved = self.contracts_saved.lock().unwrap();
        let mut tmp = None;
        std::mem::swap(&mut tmp, &mut *contracts_saved);
        std::mem::swap(&mut *contracts, &mut tmp.unwrap());

        // let mut channels = self.channels.write().unwrap();
        // let mut channels_saved = self.channels_saved.lock().unwrap();
        // let mut tmp = None;
        // std::mem::swap(&mut tmp, &mut *channels_saved);
        // std::mem::swap(&mut *channels, &mut tmp.unwrap());
    }
}

impl Default for DlcMemoryStorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncStorage for DlcMemoryStorageProvider {
    async fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, DaemonError> {
        let map = self.contracts.read().expect("Could not get read lock");
        Ok(map.get(id).cloned())
    }

    async fn get_contracts(&self) -> Result<Vec<Contract>, DaemonError> {
        Ok(self
            .contracts
            .read()
            .expect("Could not get read lock")
            .values()
            .cloned()
            .collect())
    }

    async fn create_contract(&self, contract: &OfferedContract) -> Result<(), DaemonError> {
        let mut map = self.contracts.write().expect("Could not get write lock");
        let res = map.insert(contract.id, Contract::Offered(contract.clone()));
        match res {
            None => Ok(()),
            Some(_) => Err(DaemonError::StorageError(
                "Contract already exists".to_string(),
            )),
        }
    }

    async fn delete_contract(&self, id: &ContractId) -> Result<(), DaemonError> {
        let mut map = self.contracts.write().expect("Could not get write lock");
        map.remove(id);
        Ok(())
    }

    async fn update_contract(&self, contract: &Contract) -> Result<(), DaemonError> {
        let mut map = self.contracts.write().expect("Could not get write lock");
        match contract {
            a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                if let Some(temporary_contract) = map.get(&a.get_temporary_id()) {
                    map.remove(&a.get_temporary_id());
                } else {
                    panic!("Temporary contract is null");
                } 
            }
            _ => {}
        };
        map.insert(contract.get_id(), contract.clone());
        Ok(())
    }

    async fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, DaemonError> {
        let map = self.contracts.read().expect("Could not get read lock");

        let mut res: Vec<SignedContract> = Vec::new();

        for (_, val) in map.iter() {
            if let Contract::Signed(c) = val {
                res.push(c.clone());
            }
        }

        Ok(res)
    }

    async fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, DaemonError> {
        let map = self.contracts.read().expect("Could not get read lock");

        let mut res: Vec<SignedContract> = Vec::new();

        for (_, val) in map.iter() {
            if let Contract::Confirmed(c) = val {
                res.push(c.clone());
            }
        }

        Ok(res)
    }

    async fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, DaemonError> {
        let map = self.contracts.read().expect("Could not get read lock");

        let mut res: Vec<OfferedContract> = Vec::new();

        for (_, val) in map.iter() {
            if let Contract::Offered(c) = val {
                res.push(c.clone());
            }
        }

        Ok(res)
    }

    async fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, DaemonError> {
        let map = self.contracts.read().expect("Could not get read lock");

        let mut res: Vec<PreClosedContract> = Vec::new();

        for (_, val) in map.iter() {
            if let Contract::PreClosed(c) = val {
                res.push(c.clone());
            }
        }
        Ok(res)
    }
}
