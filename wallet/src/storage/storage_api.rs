extern crate tokio;
extern crate base64;
use dlc_clients::{StorageApiClient, ApiError, NewContract, UpdateContract};
use log::info;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};
use dlc_manager::{ContractId, Storage};
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::error::Error;
use tokio::runtime::Runtime;

use super::utils::{deserialize_contract, serialize_contract, get_contract_state_str};

pub struct StorageApiProvider {

    client: StorageApiClient,

    runtime: Runtime,

    contract_mutexes: Arc<Mutex<HashMap<String, Mutex<()>>>>

}

impl StorageApiProvider {
    
    pub fn new() -> Self {
        let contract_mutexes: Arc<Mutex<HashMap<String, Mutex<()>>>> = Arc::new(Mutex::new(HashMap::new()));
        let storage_api_endpoint: String = env::var("STORAGE_API_ENDPOINT").unwrap_or("http://localhost:8100".to_string());
        Self { client: StorageApiClient::new(storage_api_endpoint), runtime: Runtime::new().unwrap(), contract_mutexes: contract_mutexes }
    }

    pub fn delete_contracts(&self) {
        let _res = self.runtime.block_on(self.client.delete_contracts());
    }

    pub fn get_contracts_by_state(&self, state: String) -> Result<Vec<Contract>, Error> {
        let contracts_res: Result<Vec<dlc_clients::Contract>, ApiError> = self.runtime.block_on(self.client.get_contracts_by_state(state.clone()));
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
            let contract_res : Result<Option<dlc_clients::Contract>, ApiError> = self.runtime.block_on(self.client.get_contract(cid.clone()));
            let unw_contract = contract_res.unwrap();
            if unw_contract.is_some() {
                let bytes = base64::decode(unw_contract.unwrap().content).unwrap();
                let contract = deserialize_contract(&bytes)?;
                Ok(Some(contract))
            } else {
                info!("Contract not found with id: {}", cid);
                Ok(None)
            }
    }

    fn get_contracts(&self) -> Result<Vec<Contract>, Error> {
        let contracts_res : Result<Vec<dlc_clients::Contract>, ApiError> = self.runtime.block_on(self.client.get_contracts());
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

    fn create_contract(&mut self, contract: &OfferedContract) -> Result<(), Error> {
        let data = serialize_contract(&Contract::Offered(contract.clone()))?;
            let bytes = contract.id.to_vec();
            let uuid = base64::encode(&bytes);
            let mut mutex = self.contract_mutexes.lock().unwrap();
            let lock = mutex.entry(uuid.clone()).or_insert(Mutex::new(()));
            let _guard = lock.lock();
            let req = NewContract{uuid: uuid.clone(), state: "offered".to_string(), content: base64::encode(&data)};
            let _result = self.runtime.block_on(self.client.create_contract(req));
            Ok(())
    }

    fn delete_contract(&mut self, id: &ContractId) -> Result<(), Error> {
        let bytes = id.to_vec();
            let cid = base64::encode(&bytes);
            let mut mutex = self.contract_mutexes.lock().unwrap();
            let lock = mutex.entry(cid.clone()).or_insert(Mutex::new(()));
            let _guard = lock.lock();
            let _result = self.runtime.block_on(self.client.delete_contract(cid.clone()));
            Ok(())
    }

    fn update_contract(&mut self, contract: &Contract) -> Result<(), Error> {
        let c_id = contract.get_id();
            let bytes = c_id.to_vec();
            let contract_id : String = base64::encode(&bytes);
            let curr_state = get_contract_state_str(contract);
            match contract {
                a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                    let _res = self.delete_contract(&a.get_temporary_id());
                }
                _ => {}
            };
            let contract_res : Result<Option<dlc_clients::Contract>, ApiError> = self.runtime.block_on(self.client.get_contract(contract_id.clone()));
            let unw_contract = contract_res.unwrap();
            let data = serialize_contract(contract).unwrap();
            let encoded_content = base64::encode(&data);
            if unw_contract.is_some() {
                let _res = self.runtime.block_on(self.client.update_contract(contract_id.clone(), UpdateContract{state: Some(curr_state.clone()), content: Some(encoded_content)}));
                Ok(())
            } else {
                let _res = self.runtime.block_on(self.client.create_contract(NewContract{ uuid: contract_id.clone(), state: curr_state.clone(), content: encoded_content}));
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
        let contracts_per_state = self.get_contracts_by_state("confirmed".to_string()).unwrap();
            let mut res: Vec<SignedContract> = Vec::new();
            for val in contracts_per_state {
                if let Contract::Confirmed(c) = val {
                    res.push(c.clone());
                }
            }
            return Ok(res);
    }

    fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("pre_closed".to_string()).unwrap();
            let mut res: Vec<PreClosedContract> = Vec::new();
            for val in contracts_per_state {
                if let Contract::PreClosed(c) = val {
                    res.push(c.clone());
                }
            }
            return Ok(res);
    }
}
