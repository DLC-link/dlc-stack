extern crate tokio;
extern crate base64;
use log::info;
use std::env;
use dlc_manager::{ContractId, Storage};
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::error::Error;
use dlc_sled_storage_provider::SledStorageProvider;
use crate::storage::memory_storage::MemoryStorage;
use super::storage_api::StorageApiProvider;

pub struct StorageProvider {

    memory_storage: MemoryStorage,

    sled_storage: Option<SledStorageProvider>,

    storage_api: Option<StorageApiProvider>,

}

impl StorageProvider {

    pub fn new() -> Self {
        let memory_storage = MemoryStorage::new();
        let use_storage_api: bool = env::var("STORAGE_API_ENABLED")
            .unwrap_or("false".to_string())
            .parse().unwrap();
        let use_sled: bool = env::var("SLED_ENABLED")
            .unwrap_or("false".to_string())
            .parse().unwrap();
        let sled_path: String = env::var("SLED_PATH").unwrap_or("contracts_db".to_string());
        if use_storage_api {
            info!("Storage API enabled: {}", use_storage_api);
            Self {memory_storage: memory_storage, sled_storage: None, storage_api: Some(StorageApiProvider::new())}
        } else if use_sled {
            info!("Sled enabled: {}", use_sled);
            Self {memory_storage: memory_storage, sled_storage: Some(SledStorageProvider::new(sled_path.as_str()).unwrap()), storage_api: None}
        } else {
            Self {memory_storage: memory_storage, sled_storage: None, storage_api: None}
        }
    }

    pub fn delete_contracts(&self) {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().delete_contracts();
        } else if self.sled_storage.is_some() {
            // TODO
        } else {
            self.memory_storage.delete_contracts()
        }
    }
}

impl Default for StorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for StorageProvider {
    
    fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().get_contract(id)
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_ref().unwrap().get_contract(id)
        } else {
            self.memory_storage.get_contract(id)
        }
    }

    fn get_contracts(&self) -> Result<Vec<Contract>, Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().get_contracts()
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_ref().unwrap().get_contracts()
        } else {
            self.memory_storage.get_contracts()
        }
    }

    fn create_contract(&mut self, contract: &OfferedContract) -> Result<(), Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_mut().unwrap().create_contract(contract)
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_mut().unwrap().create_contract(contract)
        } else {
            self.memory_storage.create_contract(contract)
        }
    }

    fn delete_contract(&mut self, id: &ContractId) -> Result<(), Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_mut().unwrap().delete_contract(id)
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_mut().unwrap().delete_contract(id)
        } else {
            self.memory_storage.delete_contract(id)
        }
    }

    fn update_contract(&mut self, contract: &Contract) -> Result<(), Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_mut().unwrap().update_contract(contract)
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_mut().unwrap().update_contract(contract)
        } else {
            self.memory_storage.update_contract(contract)
        }
    }

    fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().get_contract_offers()
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_ref().unwrap().get_contract_offers()
        } else {
            self.memory_storage.get_contract_offers()
        }
    }

    fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().get_signed_contracts()
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_ref().unwrap().get_signed_contracts()
        } else {
            self.memory_storage.get_signed_contracts()
        }
    }

    fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().get_confirmed_contracts()
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_ref().unwrap().get_confirmed_contracts()
        } else {
            self.memory_storage.get_confirmed_contracts()
        }
    }

    fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error> {
        if self.storage_api.is_some() {
            self.storage_api.as_ref().unwrap().get_preclosed_contracts()
        } else if self.sled_storage.is_some() {
            self.sled_storage.as_ref().unwrap().get_preclosed_contracts()
        } else {
            self.memory_storage.get_preclosed_contracts()
        }
    }
}
