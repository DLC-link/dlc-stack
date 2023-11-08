use dlc_link_manager::AsyncStorage;
use dlc_manager::contract::offered_contract::OfferedContract;
use dlc_manager::contract::signed_contract::SignedContract;
use dlc_manager::contract::Contract as DlcContract;
use dlc_manager::contract::PreClosedContract;
use dlc_manager::error::Error;
use dlc_manager::ContractId;
use secp256k1_zkp::SecretKey;

use crate::utils::{get_contract_id_string, to_storage_error};
use crate::{
    ApiError, Contract, ContractRequestParams, ContractsRequestParams, NewContract,
    StorageApiClient, UpdateContract,
};

use super::utils::{deserialize_contract, get_contract_state_str, serialize_contract};

pub struct AsyncStorageApiProvider {
    client: StorageApiClient,
    public_key: String,
    secret_key: SecretKey, // hand in private and pub key, and do the signing here?
}

impl AsyncStorageApiProvider {
    pub fn new(public_key: String, secret_key: SecretKey, storage_api_endpoint: String) -> Self {
        Self {
            client: StorageApiClient::new(storage_api_endpoint),
            public_key,
            secret_key,
        }
    }

    // // TODO: For testing only, delete before production
    // pub async fn delete_contracts(&self) {
    //     let _res = self.client.delete_contracts(self.key.clone());
    // }

    pub async fn get_contracts_by_state(&self, state: String) -> Result<Vec<DlcContract>, Error> {
        let contracts_res = self
            .client
            .get_contracts(
                ContractsRequestParams {
                    state: Some(state),
                    key: self.public_key.clone(),
                    uuid: None,
                },
                self.secret_key,
            )
            .await
            .map_err(to_storage_error)?;
        let mut contents: Vec<String> = vec![];
        let mut contracts: Vec<DlcContract> = vec![];
        for c in contracts_res {
            // don't unwrap here, as it will kill the thread
            contents.push(c.content);
        }
        for c in contents {
            let bytes = base64::decode(c.clone()).map_err(to_storage_error)?;
            let contract = deserialize_contract(&bytes).map_err(to_storage_error)?;
            contracts.push(contract);
        }
        Ok(contracts)
    }
}

impl AsyncStorage for AsyncStorageApiProvider {
    async fn get_contract(&self, id: &ContractId) -> Result<Option<DlcContract>, Error> {
        let cid = get_contract_id_string(*id);
        let contract_res = self
            .client
            .get_contract(
                ContractRequestParams {
                    key: self.public_key.clone(),
                    uuid: cid.clone(),
                },
                self.secret_key,
            )
            .await
            .map_err(to_storage_error)?;
        match contract_res {
            Some(res) => {
                let bytes = base64::decode(res.content).map_err(to_storage_error)?;
                let contract = deserialize_contract(&bytes).map_err(to_storage_error)?;
                Ok(Some(contract))
            }
            _ => Ok(None),
        }
    }

    async fn get_contracts(&self) -> Result<Vec<DlcContract>, Error> {
        let contracts_res: Result<Vec<Contract>, ApiError> = self
            .client
            .get_contracts(
                ContractsRequestParams {
                    key: self.public_key.clone(),
                    uuid: None,
                    state: None,
                },
                self.secret_key,
            )
            .await;
        let mut contents: Vec<String> = vec![];
        let mut contracts: Vec<DlcContract> = vec![];
        let unpacked_contracts = contracts_res.map_err(to_storage_error)?;
        for c in unpacked_contracts {
            contents.push(c.content);
        }
        for c in contents {
            let bytes = base64::decode(c.clone()).map_err(to_storage_error)?;
            let contract = deserialize_contract(&bytes).map_err(to_storage_error)?;
            contracts.push(contract);
        }
        Ok(contracts)
    }

    async fn create_contract(&self, contract: &OfferedContract) -> Result<(), Error> {
        let data = serialize_contract(&DlcContract::Offered(contract.clone()))?;
        let uuid = get_contract_id_string(contract.id);
        let req = NewContract {
            uuid: uuid.clone(),
            state: "offered".to_string(),
            content: base64::encode(&data),
            key: self.public_key.clone(),
        };
        self.client
            .create_contract(req, self.secret_key)
            .await
            .map_err(to_storage_error)?;
        Ok(())
    }

    async fn delete_contract(&self, id: &ContractId) -> Result<(), Error> {
        let cid = get_contract_id_string(*id);
        self.client
            .delete_contract(
                ContractRequestParams {
                    key: self.public_key.clone(),
                    uuid: cid.clone(),
                },
                self.secret_key,
            )
            .await
            .map_err(to_storage_error)?;
        Ok(())
    }

    async fn update_contract(&self, contract: &DlcContract) -> Result<(), Error> {
        match contract {
            a @ DlcContract::Accepted(_) | a @ DlcContract::Signed(_) => {
                let _ = self.delete_contract(&a.get_temporary_id()).await;
                match self
                    .client
                    .update_contract(
                        UpdateContract {
                            uuid: get_contract_id_string(contract.get_id()),
                            state: Some(get_contract_state_str(contract)),
                            content: Some(base64::encode(serialize_contract(contract)?)),
                            key: self.public_key.clone(),
                        },
                        self.secret_key,
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(_) => {
                        self.client
                            .create_contract(
                                NewContract {
                                    uuid: get_contract_id_string(contract.get_id()),
                                    state: get_contract_state_str(contract),
                                    content: base64::encode(serialize_contract(contract)?),
                                    key: self.public_key.clone(),
                                },
                                self.secret_key,
                            )
                            .await
                            .map_err(to_storage_error)?;
                    }
                }
                Ok(())
            }
            _ => {
                self.client
                    .update_contract(
                        UpdateContract {
                            uuid: get_contract_id_string(contract.get_id()),
                            state: Some(get_contract_state_str(contract)),
                            content: Some(base64::encode(serialize_contract(contract)?)),
                            key: self.public_key.clone(),
                        },
                        self.secret_key,
                    )
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
            if let DlcContract::Offered(c) = val {
                res.push(c.clone());
            }
        }
        Ok(res)
    }

    async fn get_signed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("signed".to_string()).await?;
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let DlcContract::Signed(c) = val {
                res.push(c.clone());
            }
        }
        Ok(res)
    }

    async fn get_confirmed_contracts(&self) -> Result<Vec<SignedContract>, Error> {
        let contracts_per_state = self.get_contracts_by_state("confirmed".to_string()).await?;
        let mut res: Vec<SignedContract> = Vec::new();
        for val in contracts_per_state {
            if let DlcContract::Confirmed(c) = val {
                res.push(c.clone());
            }
        }
        Ok(res)
    }

    async fn get_preclosed_contracts(&self) -> Result<Vec<PreClosedContract>, Error> {
        let contracts_per_state = self
            .get_contracts_by_state("pre_closed".to_string())
            .await?;
        let mut res: Vec<PreClosedContract> = Vec::new();
        for val in contracts_per_state {
            if let DlcContract::PreClosed(c) = val {
                res.push(c.clone());
            }
        }
        Ok(res)
    }
}
