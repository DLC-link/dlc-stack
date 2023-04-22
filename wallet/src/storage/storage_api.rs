extern crate base64;
extern crate tokio;
use dlc_clients::{ApiError, NewContract, StorageApiClient, UpdateContract};
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::error::Error;
use dlc_manager::{ContractId, Storage};
use log::{info, warn};
use std::env;
use tokio::runtime::Runtime;

use crate::storage::utils::{get_contract_id_string, to_storage_error};

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
        let contracts_res: Result<Vec<dlc_clients::Contract>, ApiError> = self
            .runtime
            .block_on(self.client.get_contracts_by_state(state.clone()));
        let mut contents: Vec<String> = vec![];
        let mut contracts: Vec<Contract> = vec![];
        for c in contracts_res.unwrap() {
            contents.push(c.content);
        }
        for c in contents {
            let bytes = base64::decode(c.clone()).map_err(to_storage_error)?;
            let contract = deserialize_contract(&bytes)?;
            contracts.push(contract);
        }
        Ok(contracts)
    }
}

impl Storage for StorageApiProvider {
    fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, Error> {
        let cid = get_contract_id_string(*id);
        info!("Get contract by id - {}", cid.clone());
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> =
            self.runtime.block_on(self.client.get_contract(cid.clone()));
        if let Some(res) = contract_res.map_err(to_storage_error)? {
            let bytes = base64::decode(res.content).unwrap();
            let contract = deserialize_contract(&bytes)?;
            Ok(Some(contract))
        } else {
            info!("Contract not found with id: {}", cid.clone());
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

    fn create_contract(self: &StorageApiProvider, contract: &OfferedContract) -> Result<(), Error> {
        let data = serialize_contract(&Contract::Offered(contract.clone()))?;
        let uuid = get_contract_id_string(contract.id);
        info!("Create new contract with contract id {}", uuid.clone());
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

    fn delete_contract(self: &StorageApiProvider, id: &ContractId) -> Result<(), Error> {
        let cid = get_contract_id_string(*id);
        info!("Delete contract with contract id {}", cid.clone());
        let res = self
            .runtime
            .block_on(self.client.delete_contract(cid.clone()));
        match res {
            Ok(r) => {
                info!(
                    "Contract has been successfully deleted with id {}",
                    cid.clone()
                );
                return Ok(r);
            }
            Err(err) => {
                info!("Contract deletion has been failed with id {}", cid.clone());
                return Err(to_storage_error(err));
            }
        }
    }

    fn update_contract(self: &StorageApiProvider, contract: &Contract) -> Result<(), Error> {
        let contract_id: String = get_contract_id_string(contract.get_id());
        let curr_state = get_contract_state_str(contract);
        info!(
            "Update contract with contract id {} - state: {}",
            contract_id.clone(),
            curr_state.clone()
        );
        match contract {
            a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                let res = self.delete_contract(&a.get_temporary_id());
                let del_con_id = get_contract_id_string(a.get_temporary_id());
                match res {
                    Ok(_) => {
                        info!("Contract has been successfully deleted (during update) with id {} and state '{}'", del_con_id, curr_state.clone());
                    }
                    Err(err) => {
                        warn!("Deleting contract has been failed (during update) with id {} and state '{}'", del_con_id, curr_state.clone());
                        return Err(to_storage_error(err));
                    }
                }
            }
            _ => {}
        };
        info!(
            "Get contract with contract id {} before updating (state: {}) ...",
            contract_id.clone(),
            curr_state.clone()
        );
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> = self
            .runtime
            .block_on(self.client.get_contract(contract_id.clone()));
        let unw_contract = match contract_res {
            Ok(res) => {
                info!(
                    "Contract has been gathered sucessfully with id {}.",
                    contract_id.clone()
                );
                res
            }
            Err(api_err) => {
                if api_err.status == 404 {
                    info!(
                        "Not found API error has been thrown by storage API with contract id {}",
                        contract_id.clone()
                    );
                    None
                } else {
                    info!(
                        "Cannot get contract with id {} by storage API",
                        contract_id.clone()
                    );
                    return Err(to_storage_error(api_err));
                }
            }
        };
        let data = serialize_contract(contract).unwrap();
        let encoded_content = base64::encode(&data);
        if unw_contract.is_some() {
            info!(
                "As contract exists with contract id {}, update contract (to state '{}')",
                contract_id.clone(),
                curr_state.clone()
            );
            let update_res = self.runtime.block_on(self.client.update_contract(
                contract_id.clone(),
                UpdateContract {
                    state: Some(curr_state.clone()),
                    content: Some(encoded_content),
                },
            ));
            match update_res {
                Ok(_) => {
                    info!(
                        "Contract has been successfully updated with id {} and state '{}'",
                        contract_id.clone(),
                        curr_state.clone()
                    );
                    return Ok(());
                }
                Err(err) => {
                    info!(
                        "Contract update has been failed with id {}, state: {}",
                        contract_id.clone(),
                        curr_state.clone()
                    );
                    return Err(to_storage_error(err));
                }
            }
        } else {
            info!(
                "As contract does not exist with contract id {}, create contract (with state '{}')",
                contract_id.clone(),
                curr_state.clone()
            );
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
                        "Contract creation has been failed (during update) with id {}, state: {}",
                        contract_id.clone(),
                        curr_state.clone()
                    );
                    return Err(to_storage_error(err));
                }
            }
        }
    }

    fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("offered".to_string())?; // TODO: If we're already getting by state, why do we need to also match on type later?
        let mut res: Vec<OfferedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Offered(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("signed".to_string())?;
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Signed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("confirmed".to_string())?;
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Confirmed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("pre_closed".to_string())?;
        let mut res: Vec<PreClosedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::PreClosed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    fn upsert_channel(
        &self,
        _channel: dlc_manager::channel::Channel,
        _contract: Option<Contract>,
    ) -> Result<(), Error> {
        todo!()
    }

    fn delete_channel(&self, _channel_id: &dlc_manager::ChannelId) -> Result<(), Error> {
        todo!()
    }

    fn get_channel(
        &self,
        _channel_id: &dlc_manager::ChannelId,
    ) -> Result<Option<dlc_manager::channel::Channel>, Error> {
        todo!()
    }

    fn get_signed_channels(
        &self,
        _channel_state: Option<dlc_manager::channel::signed_channel::SignedChannelStateType>,
    ) -> Result<Vec<dlc_manager::channel::signed_channel::SignedChannel>, Error> {
        todo!()
    }

    fn get_offered_channels(
        &self,
    ) -> Result<Vec<dlc_manager::channel::offered_channel::OfferedChannel>, Error> {
        todo!()
    }

    fn persist_chain_monitor(
        &self,
        _monitor: &dlc_manager::chain_monitor::ChainMonitor,
    ) -> Result<(), Error> {
        todo!()
    }

    fn get_chain_monitor(&self) -> Result<Option<dlc_manager::chain_monitor::ChainMonitor>, Error> {
        todo!()
    }
}
