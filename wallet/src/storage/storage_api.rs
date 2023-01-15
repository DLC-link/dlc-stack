extern crate base64;
extern crate tokio;
use dlc_clients::{ApiError, NewContract, StorageApiClient, UpdateContract};
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::error::Error;
use dlc_manager::{ContractId, Storage};
use log::{debug, info};
use std::sync::MutexGuard;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;

use crate::storage::utils::to_storage_error;

use super::utils::{deserialize_contract, get_contract_state_str, serialize_contract};

pub struct StorageApiProvider {
    client: StorageApiClient,

    runtime: Runtime,

    contract_mutexes: Arc<HashMap<String, ValueMutex<String>>>,
}

impl StorageApiProvider {
    pub fn new() -> Self {
        info!("Creating storage API provider");
        let storage_api_endpoint: String =
            env::var("STORAGE_API_ENDPOINT").unwrap_or("http://localhost:8100".to_string());
        Self {
            client: StorageApiClient::new(storage_api_endpoint),
            runtime: Runtime::new().unwrap(),
            contract_mutexes: Arc::new(HashMap::new()),
        }
    }

    pub fn delete_contracts(&self) {
        info!("Delete all contracts by storage api ...");
        let _res = self.runtime.block_on(self.client.delete_contracts());
    }

    pub fn get_contracts_by_state(&self, state: String) -> Result<Vec<Contract>, Error> {
        debug!("Get contracts by state - {}", state.clone());
        let contracts_res: Result<Vec<dlc_clients::Contract>, ApiError> = self
            .runtime
            .block_on(self.client.get_contracts_by_state(state.clone()));
        let mut contents: Vec<String> = vec![];
        let mut contracts: Vec<Contract> = vec![];
        for c in contracts_res.unwrap() {
            contents.push(c.content);
        }
        for c in contents {
            let bytes = base64::decode(c.clone()).unwrap();
            let contract = deserialize_contract(&bytes).unwrap();
            contracts.push(contract);
        }
        Ok(contracts)
    }
}

impl Storage for StorageApiProvider {
    fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, Error> {
        let bytes = id.to_vec();
        let cid = base64::encode(&bytes);
        info!("Get contract by id (base64 encoded) - {}", cid.clone());
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> =
            self.runtime.block_on(self.client.get_contract(cid.clone()));
        if let Some(res) = contract_res.map_err(to_storage_error)? {
            let bytes = base64::decode(res.content).unwrap();
            let contract = deserialize_contract(&bytes)?;
            Ok(Some(contract))
        } else {
            info!(
                "Contract not found with id: {} (base64 encoded)",
                cid.clone()
            );
            Ok(None)
        }
    }

    fn get_contracts(&self) -> Result<Vec<Contract>, Error> {
        let contracts_res: Result<Vec<dlc_clients::Contract>, ApiError> =
            self.runtime.block_on(self.client.get_contracts());
        let mut contents: Vec<String> = vec![];
        let mut contracts: Vec<Contract> = vec![];
        let unpacked_contracts = contracts_res.map_err(to_storage_error)?;
        for c in unpacked_contracts {
            contents.push(c.content);
        }
        for c in contents {
            let bytes = base64::decode(c.clone()).unwrap();
            let contract = deserialize_contract(&bytes).unwrap();
            contracts.push(contract);
        }
        Ok(contracts)
    }

    fn create_contract(&mut self, contract: &OfferedContract) -> Result<(), Error> {
        let data = serialize_contract(&Contract::Offered(contract.clone()))?;
        let bytes = contract.id.to_vec();
        let uuid = base64::encode(&bytes);
        info!(
            "Create new contract with contract id {} (base64 encoded)",
            uuid.clone()
        );
        // lock by contract id
        let mutexes = Arc::get_mut(&mut self.contract_mutexes).unwrap();
        let mutex = mutexes
            .entry(uuid.clone())
            .or_insert_with(|| ValueMutex::new(uuid.clone()));
        let guard = mutex.lock(uuid.clone());

        // TODO: workaround to avoid duplication error - but expect any other error 
        info!(
            "Get contract by id (base64 encoded) - {} (before creating contract)",
            uuid.clone()
        );
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> = self
            .runtime
            .block_on(self.client.get_contract(uuid.clone()));
        if let Some(_) = contract_res.map_err(to_storage_error)? {
            info!(
                "Contract with id '{}' would be created again. Skipping ...",
                uuid.clone()
            );
            return Ok(());
        } else {
            info!(
                "Contract not found with id: {} (base64 encoded) before creating contract",
                uuid.clone()
            );
        }

        let req = NewContract {
            uuid: uuid.clone(),
            state: "offered".to_string(),
            content: base64::encode(&data),
        };
        let res = self.runtime.block_on(self.client.create_contract(req));
        match res {
            Ok(_) => {
                drop(guard);
                mutexes.remove(&uuid);
                return Ok(());
            }
            Err(err) => {
                drop(guard);
                mutexes.remove(&uuid);
                return Err(to_storage_error(err));
            }
        }
    }

    fn delete_contract(&mut self, id: &ContractId) -> Result<(), Error> {
        let bytes = id.to_vec();
        let cid = base64::encode(&bytes);
        info!(
            "Delete contract with contract id {} (base64 encoded)",
            cid.clone()
        );
        // lock by contract id
        let mutexes = Arc::get_mut(&mut self.contract_mutexes).unwrap();
        let mutex = mutexes
            .entry(cid.clone())
            .or_insert_with(|| ValueMutex::new(cid.clone()));
        let guard = mutex.lock(cid.clone());

        let res = self
            .runtime
            .block_on(self.client.delete_contract(cid.clone()));
        match res {
            Ok(r) => {
                drop(guard);
                mutexes.remove(&cid);
                return Ok(r);
            }
            Err(err) => {
                drop(guard);
                mutexes.remove(&cid);
                return Err(to_storage_error(err));
            }
        }
    }

    fn update_contract(&mut self, contract: &Contract) -> Result<(), Error> {
        let c_id = contract.get_id();
        let bytes = c_id.to_vec();
        let contract_id: String = base64::encode(&bytes);
        info!(
            "Update contract with contract id {} (base64 encoded)",
            contract_id.clone()
        );
        let curr_state = get_contract_state_str(contract);
        match contract {
            a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                let _res = self.delete_contract(&a.get_temporary_id());
            }
            _ => {}
        };
        info!(
            "Get contract with contract id {} (base64 encoded) before updating ...",
            contract_id.clone()
        );
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> = self
            .runtime
            .block_on(self.client.get_contract(contract_id.clone()));
        let unw_contract = contract_res.unwrap();
        let data = serialize_contract(contract).unwrap();
        let encoded_content = base64::encode(&data);
        if unw_contract.is_some() {
            info!(
                "As contract exists with contract id {} (base64 encoded), update contract ...",
                contract_id.clone()
            );
            let _res = self.runtime.block_on(self.client.update_contract(
                contract_id.clone(),
                UpdateContract {
                    state: Some(curr_state.clone()),
                    content: Some(encoded_content),
                },
            ));
            Ok(())
        } else {
            info!("As contract does not exist with contract id {} (base64 encoded), create contract ...", contract_id.clone());
            let _res = self
                .runtime
                .block_on(self.client.create_contract(NewContract {
                    uuid: contract_id.clone(),
                    state: curr_state.clone(),
                    content: encoded_content,
                }));
            Ok(())
        }
    }

    fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("offered".to_string()).unwrap();
        let mut res: Vec<OfferedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Offered(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("signed".to_string()).unwrap();
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Signed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self
            .get_contracts_by_state("confirmed".to_string())
            .unwrap();
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Confirmed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error> {
        let contracts_per_state = self
            .get_contracts_by_state("pre_closed".to_string())
            .unwrap();
        let mut res: Vec<PreClosedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::PreClosed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }
}

struct ValueMutex<T> {
    value: T,
    mutex: Mutex<()>,
}

impl<T: Eq + Clone> ValueMutex<T> {
    fn new(value: T) -> Self {
        ValueMutex {
            value,
            mutex: Mutex::new(()),
        }
    }

    fn lock(&self, value: T) -> MutexGuard<'_, ()> {
        if self.value == value {
            self.mutex.lock().unwrap()
        } else {
            panic!("Locked with a different value")
        }
    }
}
