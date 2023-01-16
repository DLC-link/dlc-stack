extern crate base64;
extern crate tokio;
use dlc_clients::{ApiError, NewContract, StorageApiClient, UpdateContract};
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::error::Error;
use dlc_manager::{ContractId, Storage};
use log::{debug, info, warn};
use std::env;
use tokio::runtime::Runtime;

use crate::storage::utils::to_storage_error;

use super::utils::{deserialize_contract, get_contract_state_str, serialize_contract};

pub struct StorageApiProvider {
    client: StorageApiClient,

    runtime: Runtime,
}

impl StorageApiProvider {
    pub fn new() -> Self {
        info!("Creating storage API provider");
        let storage_api_endpoint: String =
            env::var("STORAGE_API_ENDPOINT").unwrap_or("http://localhost:8100".to_string());
        Self {
            client: StorageApiClient::new(storage_api_endpoint),
            runtime: Runtime::new().unwrap(),
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
        let req = NewContract {
            uuid: uuid.clone(),
            state: "offered".to_string(),
            content: base64::encode(&data),
        };
        let res = self.runtime.block_on(self.client.create_contract(req));
        match res {
            Ok(_) => {
                info!(
                    "Contract has been successfully created with id {} and state 'offered'",
                    uuid.clone()
                );
                return Ok(());
            }
            Err(err) => {
                info!("Contract creation has failed with id {}", uuid.clone());
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
        let res = self
            .runtime
            .block_on(self.client.delete_contract(cid.clone()));
        match res {
            Ok(r) => {
                info!(
                    "Contract has been successfully deleted with id {} and state 'offered'",
                    cid.clone()
                );
                return Ok(r);
            }
            Err(err) => {
                info!(
                    "Contract deletion has been failed with id {} and state 'offered'",
                    cid.clone()
                );
                return Err(to_storage_error(err));
            }
        }
    }

    fn update_contract(&mut self, contract: &Contract) -> Result<(), Error> {
        let c_id = contract.get_id();
        let bytes = c_id.to_vec();
        let contract_id: String = base64::encode(&bytes);
        let curr_state = get_contract_state_str(contract);
        info!(
            "Update contract with contract id {} (base64 encoded) - state: {}",
            contract_id.clone(),
            curr_state.clone()
        );
        match contract {
            a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                let res = self.delete_contract(&a.get_temporary_id());
                let del_vec = &a.get_temporary_id().to_vec();
                let del_con_id = base64::encode(&del_vec);
                match res {
                    Ok(_) => {
                        info!("Contract has been successfully deleted (during update) with id {} (base64 encoded) and state 'offered'", del_con_id);
                    }
                    Err(err) => {
                        warn!("Deleting contract has been failed (during update) with id {} (base64 encoded) and state '{}'", del_con_id, curr_state.clone());
                        return Err(to_storage_error(err));
                    }
                }
            }
            _ => {}
        };
        info!(
            "Get contract with contract id {} (base64 encoded) before updating (state: {}) ...",
            contract_id.clone(),
            curr_state.clone()
        );
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> = self
            .runtime
            .block_on(self.client.get_contract(contract_id.clone()));
        let unw_contract = match contract_res {
            Ok(res) => res,
            Err(api_err) => {
                if api_err.status == 404 {
                    info!(
                        "Not found API error has been thrown by storage API with contract id {}",
                        contract_id.clone()
                    );
                    None
                } else {
                    return Err(to_storage_error(api_err));
                }
            }
        };
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
            let create_res = self
                .runtime
                .block_on(self.client.create_contract(NewContract {
                    uuid: contract_id.clone(),
                    state: curr_state.clone(),
                    content: encoded_content,
                }));
            match create_res {
                Ok(_) => {
                    info!(
                        "Contract has been successfully created with id {} and state '{}'",
                        contract_id.clone(),
                        curr_state.clone()
                    );
                    return Ok(());
                }
                Err(err) => {
                    info!(
                        "Contract creation has been failed (during update) with id {}, staate: {}",
                        contract_id.clone(),
                        curr_state.clone()
                    );
                    return Err(to_storage_error(err));
                }
            }
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
