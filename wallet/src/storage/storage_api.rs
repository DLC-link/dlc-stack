extern crate base64;
extern crate tokio;
use dlc_clients::{
    ApiError, ContractRequestParams, ContractsRequestParams, NewContract, StorageApiClient,
    UpdateContract,
};
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::error::Error;
use dlc_manager::{ContractId, Storage};
use log::{debug, info, warn};
use std::env;
use tokio::runtime::Runtime;

use crate::storage::utils::{get_contract_id_string, to_storage_error};

use super::utils::{deserialize_contract, get_contract_state_str, serialize_contract};

pub struct StorageApiProvider {
    client: StorageApiClient,
    key: String,
    runtime: Runtime,
}

impl StorageApiProvider {
    pub fn new(key: String) -> Self {
        info!("Creating storage API provider");
        let storage_api_endpoint: String =
            env::var("STORAGE_API_ENDPOINT").unwrap_or("http://localhost:8100".to_string());
        Self {
            client: StorageApiClient::new(storage_api_endpoint),
            key,
            runtime: Runtime::new().unwrap(),
        }
    }

    // Todo: delete later, for testing only
    pub fn delete_contracts(&self) {
        info!("Delete all contracts by storage api ...");
        let _res = self
            .runtime
            .block_on(self.client.delete_contracts(self.key.clone()));
    }

    pub fn get_contracts_by_state(&self, state: String) -> Result<Vec<Contract>, Error> {
        let contracts: Result<Vec<dlc_clients::Contract>, ApiError> =
            self.runtime
                .block_on(self.client.get_contracts(ContractsRequestParams {
                    state: Some(state.clone()),
                    key: self.key.clone(),
                    uuid: None,
                }));
        match contracts {
            Ok(contracts) => {
                Ok(contracts
                    .into_iter()
                    .map(|c| {
                        // Use Serde to automagically build this object?
                        let bytes = base64::decode(c.content).unwrap();
                        let contract = deserialize_contract(&bytes).unwrap();
                        contract
                    })
                    .collect())
            }
            Err(e) => {
                warn!("Got an error getting contracts: {:?}", e);
                Err(to_storage_error(e))
            }
        }
    }
}

impl Storage for StorageApiProvider {
    fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, Error> {
        let cid = get_contract_id_string(*id);
        info!("Get contract by id - {}", cid.clone());
        let contract = self
            .runtime
            .block_on(self.client.get_contract(ContractRequestParams {
                key: self.key.clone(),
                uuid: cid.clone(),
            }))
            .map_err(to_storage_error)?;

        match contract {
            Some(c) => {
                // Use Serde to automagically build this object?
                let bytes = base64::decode(c.content).unwrap();
                let contract = deserialize_contract(&bytes).unwrap();
                Ok(Some(contract))
            }
            None => Ok(None),
        }
    }

    fn get_contracts(&self) -> Result<Vec<Contract>, Error> {
        let contracts = self
            .runtime
            .block_on(self.client.get_contracts(ContractsRequestParams {
                state: None,
                key: self.key.clone(),
                uuid: None,
            }))
            .map_err(to_storage_error)?;

        Ok(contracts
            .into_iter()
            .map(|c| {
                let bytes = base64::decode(c.content.clone()).unwrap();
                let contract: Contract = deserialize_contract(&bytes).unwrap();
                contract
            })
            .collect())
    }

    fn create_contract(self: &StorageApiProvider, contract: &OfferedContract) -> Result<(), Error> {
        let data = serialize_contract(&Contract::Offered(contract.clone()))?;
        let uuid = get_contract_id_string(contract.id);
        info!(
            "Create new contract with contract id {} and key {}",
            uuid.clone(),
            self.key.clone()
        );
        let req = NewContract {
            uuid: uuid.clone(),
            state: "offered".to_string(),
            content: base64::encode(&data),
            key: self.key.clone(),
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
            .block_on(self.client.delete_contract(ContractRequestParams {
                key: self.key.clone(),
                uuid: cid.clone(),
            }));
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
        let state = get_contract_state_str(contract);
        let uuid = get_contract_id_string(contract.get_id());
        info!(
            "Update contract with contract id {} - state: {}",
            uuid, state
        );
        match contract {
            a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                match self.delete_contract(&a.get_temporary_id()) {
                    Ok(_) => {}
                    Err(_) => {} // This happens when the temp contract was already deleted upon moving from Offered to Accepted
                }
                // This could be replaced with an UPSERT
                match self
                    .runtime
                    .block_on(self.client.update_contract(UpdateContract {
                        uuid: get_contract_id_string(contract.get_id()),
                        state: Some(get_contract_state_str(contract)),
                        content: Some(base64::encode(serialize_contract(contract).unwrap())),
                        key: self.key.clone(),
                    })) {
                    Ok(_) => {}
                    Err(_) => {
                        self.runtime
                            .block_on(self.client.create_contract(NewContract {
                                uuid: get_contract_id_string(contract.get_id()),
                                state: get_contract_state_str(contract),
                                content: base64::encode(serialize_contract(contract).unwrap()),
                                key: self.key.clone(),
                            }))
                            .map_err(to_storage_error)?;
                    }
                }
                Ok(())
            }
            _ => {
                self.runtime
                    .block_on(self.client.update_contract(UpdateContract {
                        uuid: get_contract_id_string(contract.get_id()),
                        state: Some(get_contract_state_str(contract)),
                        content: Some(base64::encode(serialize_contract(contract).unwrap())),
                        key: self.key.clone(),
                    }))
                    .map_err(to_storage_error)?;
                Ok(())
            }
        }
    }

    fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("offered".to_string())?;
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
