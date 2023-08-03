use dlc_clients::{
    ApiError, ContractRequestParams, ContractsRequestParams, NewContract, StorageApiClient,
    UpdateContract,
};
use dlc_link_manager::AsyncStorage;
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::contract::{Contract, PreClosedContract};
use dlc_manager::error::Error;
use dlc_manager::ContractId;

use crate::storage::utils::{get_contract_id_string, to_storage_error};

use super::utils::{deserialize_contract, get_contract_state_str, serialize_contract};

pub struct AsyncStorageApiProvider {
    client: StorageApiClient,
    key: String,
}

impl AsyncStorageApiProvider {
    pub fn new(key: String, storage_api_endpoint: String) -> Self {
        Self {
            client: StorageApiClient::new(storage_api_endpoint),
            key,
        }
    }

    // TODO: For testing only, delete later
    pub async fn delete_contracts(&self) {
        let _res = self.client.delete_contracts(self.key.clone());
    }

    pub async fn get_contracts_by_state(&self, state: String) -> Result<Vec<Contract>, Error> {
        let contracts_res: Result<Vec<dlc_clients::Contract>, ApiError> = self
            .client
            .get_contracts(ContractsRequestParams {
                state: Some(state),
                key: self.key.clone(),
                uuid: None,
            })
            .await;
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

impl AsyncStorage for AsyncStorageApiProvider {
    async fn get_contract(&self, id: &ContractId) -> Result<Option<Contract>, Error> {
        let cid = get_contract_id_string(*id);
        let contract_res: Result<Option<dlc_clients::Contract>, ApiError> = self
            .client
            .get_contract(ContractRequestParams {
                key: self.key.clone(),
                uuid: cid.clone(),
            })
            .await;
        if let Some(res) = contract_res.map_err(to_storage_error)? {
            let bytes = base64::decode(res.content).unwrap();
            let contract = deserialize_contract(&bytes)?;
            Ok(Some(contract))
        } else {
            Ok(None)
        }
    }

    async fn get_contracts(&self) -> Result<Vec<Contract>, Error> {
        let contracts_res: Result<Vec<dlc_clients::Contract>, ApiError> = self
            .client
            .get_contracts(ContractsRequestParams {
                key: self.key.clone(),
                uuid: None,
                state: None,
            })
            .await;
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

    async fn create_contract(&self, contract: &OfferedContract) -> Result<(), Error> {
        let data = serialize_contract(&Contract::Offered(contract.clone()))?;
        let uuid = get_contract_id_string(contract.id);
        let req = NewContract {
            uuid: uuid.clone(),
            state: "offered".to_string(),
            content: base64::encode(&data),
            key: self.key.clone(),
        };
        let res = self.client.create_contract(req).await;
        match res {
            Ok(_) => {
                return Ok(());
            }
            Err(err) => {
                return Err(to_storage_error(err));
            }
        }
    }

    async fn delete_contract(&self, id: &ContractId) -> Result<(), Error> {
        let cid = get_contract_id_string(*id);
        let res = self
            .client
            .delete_contract(ContractRequestParams {
                key: self.key.clone(),
                uuid: cid.clone(),
            })
            .await;
        match res {
            Ok(r) => {
                return Ok(r);
            }
            Err(err) => {
                return Err(to_storage_error(err));
            }
        }
    }

    async fn update_contract(&self, contract: &Contract) -> Result<(), Error> {
        let state = get_contract_state_str(contract);
        let uuid = get_contract_id_string(contract.get_id());
        match contract {
            a @ Contract::Accepted(_) | a @ Contract::Signed(_) => {
                if let Some(_) = self.get_contract(&a.get_temporary_id()).await? {
                    self.delete_contract(&a.get_temporary_id()).await?;
                }
                if let Some(_) = self.get_contract(&contract.get_id()).await? {
                    self.client
                        .update_contract(UpdateContract {
                            uuid: get_contract_id_string(contract.get_id()),
                            state: Some(get_contract_state_str(contract)),
                            content: Some(base64::encode(serialize_contract(contract).unwrap())),
                            key: self.key.clone(),
                        })
                        .await
                        .map_err(to_storage_error)?;
                } else {
                    self.client
                        .create_contract(NewContract {
                            uuid: get_contract_id_string(contract.get_id()),
                            state: get_contract_state_str(contract),
                            content: base64::encode(serialize_contract(contract).unwrap()),
                            key: self.key.clone(),
                        })
                        .await
                        .map_err(to_storage_error)?;
                }
                Ok(())
            }
            _ => {
                self.client
                    .update_contract(UpdateContract {
                        uuid: get_contract_id_string(contract.get_id()),
                        state: Some(get_contract_state_str(contract)),
                        content: Some(base64::encode(serialize_contract(contract).unwrap())),
                        key: self.key.clone(),
                    })
                    .await
                    .map_err(to_storage_error)?;
                Ok(())
            }
        }
    }

    async fn get_contract_offers(&self) -> Result<Vec<OfferedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("offered".to_string()).await?;
        let mut res: Vec<OfferedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Offered(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    async fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("signed".to_string()).await?;
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Signed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    async fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("confirmed".to_string()).await?;
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::Confirmed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }

    async fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error> {
        let contracts_per_state = self
            .get_contracts_by_state("pre_closed".to_string())
            .await?;
        let mut res: Vec<PreClosedContract> = Vec::new();
        for val in contracts_per_state {
            if let Contract::PreClosed(c) = val {
                res.push(c.clone());
            }
        }
        return Ok(res);
    }
}
