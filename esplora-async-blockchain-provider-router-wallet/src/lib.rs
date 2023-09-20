#![feature(async_fn_in_trait)]
use bdk::esplora_client::TxStatus;
use bdk::esplora_client::{AsyncClient, Builder};
use bitcoin::consensus::Decodable;
use bitcoin::{Block, Network, Transaction, Txid};
use dlc_link_manager::AsyncBlockchain;
use dlc_manager::{error::Error, Blockchain, Utxo};

use js_interface_wallet::WalletBlockchainProvider;
use lightning::chain::chaininterface::FeeEstimator;
use reqwest::Response;

use serde::{Deserialize, Serialize};

use bdk::blockchain::esplora::EsploraBlockchain;

const REQWEST_TIMEOUT: u64 = 30;

#[derive(Serialize, Deserialize, Debug)]
struct UtxoResp {
    txid: String,
    vout: u32,
    value: u64,
    status: UtxoStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum UtxoStatus {
    Confirmed {
        confirmed: bool,
        block_height: u64,
        block_hash: String,
        block_time: u64,
    },
    Unconfirmed {
        confirmed: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UTXOSpent {
    spent: bool,
}

pub struct EsploraAsyncBlockchainProviderRouterWallet {
    host: String,
    pub blockchain: EsploraBlockchain,
    network: Network,
}

impl EsploraAsyncBlockchainProviderRouterWallet {
    pub fn new(host: String, network: Network) -> Self {
        let client_builder = Builder::new(&host).timeout(REQWEST_TIMEOUT);
        let url_client = AsyncClient::from_builder(client_builder).unwrap();
        let blockchain = EsploraBlockchain::from_client(url_client, 20);

        Self {
            host,
            blockchain,
            network,
        }
    }

    async fn get(&self, sub_url: &str) -> Result<Response, Error> {
        // If self.host doesn't end with slash, add it
        let host = if self.host.ends_with('/') {
            self.host.to_string()
        } else {
            format!("{}/", self.host)
        };
        self.blockchain
            .client()
            .get(format!("{}{}", host, sub_url))
            .send()
            .await
            .map_err(|x| {
                dlc_manager::error::Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    x,
                ))
            })
    }

    // why using a local client here, but the blockchain client above??
    async fn get_from_json<T>(&self, sub_url: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get(sub_url)
            .await
            .unwrap()
            .json::<T>()
            .await
            .map_err(|e| Error::BlockchainError(e.to_string()))
    }

    async fn get_bytes(&self, sub_url: &str) -> Result<Vec<u8>, Error> {
        let bytes = self.get(sub_url).await.unwrap().bytes().await;
        Ok(bytes
            .map_err(|e| Error::BlockchainError(e.to_string()))?
            .into_iter()
            .collect::<Vec<_>>())
    }
}

impl AsyncBlockchain for EsploraAsyncBlockchainProviderRouterWallet {
    async fn get_transaction_confirmations_async(&self, tx_id: &Txid) -> Result<u32, Error> {
        let tx_status = self
            .get_from_json::<TxStatus>(&format!("tx/{tx_id}/status"))
            .await?;
        if tx_status.confirmed {
            let block_chain_height = self.blockchain.get_height().await.unwrap() as u64;
            if let Some(block_height) = tx_status.block_height {
                return Ok((block_chain_height - block_height as u64 + 1) as u32);
            }
        }

        Ok(0)
    }

    async fn send_transaction_async(&self, tx: &Transaction) -> Result<(), Error> {
        self.blockchain
            .broadcast(tx)
            .await
            .map_err(|x| dlc_manager::error::Error::OracleError(x.to_string()))
    }

    async fn get_network_async(&self) -> Result<bitcoin::network::constants::Network, Error> {
        Ok(self.network)
    }

    async fn get_transaction_async(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        let raw_tx = self.get_bytes(&format!("tx/{tx_id}/raw")).await?;
        Transaction::consensus_decode(&mut std::io::Cursor::new(&*raw_tx))
            .map_err(|e| Error::BlockchainError(e.to_string()))
    }
}

impl Blockchain for EsploraAsyncBlockchainProviderRouterWallet {
    fn send_transaction(&self, _transaction: &Transaction) -> Result<(), Error> {
        // This is no longer used anywhere, as all calls can be the async version
        unimplemented!("use async version");
    }

    fn get_network(&self) -> Result<bitcoin::network::constants::Network, Error> {
        // This is no longer used anywhere, as all calls can be the async version
        unimplemented!("use async version");
    }

    fn get_blockchain_height(&self) -> Result<u64, Error> {
        // This is no longer used anywhere, as all calls can be the async version
        unimplemented!("use async version");
    }

    fn get_block_at_height(&self, _height: u64) -> Result<Block, Error> {
        //only used for lightning
        unimplemented!();
    }

    // really close to not needing this implementation at all. just one call inside of a utils file in rust-dlc remains after that, no need to have this complex code.
    fn get_transaction(&self, _tx_id: &Txid) -> Result<Transaction, Error> {
        unimplemented!();
    }

    fn get_transaction_confirmations(&self, _tx_id: &Txid) -> Result<u32, Error> {
        // This is no longer used anywhere, as all calls can be the async version
        unimplemented!("use async version");
    }
}

impl WalletBlockchainProvider for EsploraAsyncBlockchainProviderRouterWallet {
    fn get_utxos_for_address(&self, _address: &bitcoin::Address) -> Result<Vec<Utxo>, Error> {
        unimplemented!();
    }

    fn is_output_spent(&self, _txid: &Txid, _vout: u32) -> Result<bool, Error> {
        unimplemented!();
    }
}

impl FeeEstimator for EsploraAsyncBlockchainProviderRouterWallet {
    fn get_est_sat_per_1000_weight(
        &self,
        _confirmation_target: lightning::chain::chaininterface::ConfirmationTarget,
    ) -> u32 {
        unimplemented!()
    }
}
