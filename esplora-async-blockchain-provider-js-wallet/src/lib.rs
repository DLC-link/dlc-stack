#![feature(async_fn_in_trait)]
#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]
use bdk::esplora_client::TxStatus;
use bdk::esplora_client::{AsyncClient, Builder};
use bitcoin::consensus::Decodable;
use bitcoin::{Address, Block, Network, OutPoint, Script, Transaction, TxOut, Txid};
use dlc_link_manager::AsyncBlockchain;
use dlc_manager::{error::Error, Blockchain, Utxo};

use js_interface_wallet::WalletBlockchainProvider;
use lightning::chain::chaininterface::FeeEstimator;
use reqwest::Response;

use serde::{Deserialize, Serialize};

use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::vec;

use log::*;

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

pub struct EsploraAsyncBlockchainProviderJsWallet {
    host: String,
    pub blockchain: EsploraBlockchain,
    chain_data: Arc<Mutex<ChainCacheData>>,
    network: Network,
}

#[derive(Debug)]
struct ChainCacheData {
    utxos: RefCell<Option<Vec<Utxo>>>,
    txs: RefCell<Option<HashMap<String, Transaction>>>,
    height: RefCell<Option<u64>>,
}

impl PartialEq for ChainCacheData {
    fn eq(&self, other: &Self) -> bool {
        let borrowed_utxos1 = self.utxos.borrow();
        let utxos1 = borrowed_utxos1
            .as_ref()
            .expect("To be able to get the reference to the utxos in a comparison");
        let borrowed_utxos2 = other.utxos.borrow();
        let utxos2 = borrowed_utxos2
            .as_ref()
            .expect("To be able to get the reference to the utxos for a comparison");
        let check_1 = utxos1.iter().all(|utxo| utxos2.contains(utxo));
        let check_2 = self.txs == other.txs;
        let check_3 = self.height == other.height;
        check_1 && check_2 && check_3
    }
}

impl EsploraAsyncBlockchainProviderJsWallet {
    pub fn new(host: String, network: Network) -> Self {
        let client_builder = Builder::new(&host).timeout(REQWEST_TIMEOUT);
        let url_client = AsyncClient::from_builder(client_builder)
            .expect("To be able to create a bdk esplora client ");
        let blockchain = EsploraBlockchain::from_client(url_client, 20);

        Self {
            host,
            blockchain,
            chain_data: Arc::new(Mutex::new(ChainCacheData {
                utxos: Some(vec![]).into(),
                txs: Some(HashMap::new()).into(),
                height: Some(0).into(),
            })),
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

    // TODO: why using a local client here, but the blockchain client above??
    async fn get_from_json<T>(&self, sub_url: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get(sub_url)
            .await?
            .json::<T>()
            .await
            .map_err(|e| Error::BlockchainError(e.to_string()))
    }

    async fn get_bytes(&self, sub_url: &str) -> Result<Vec<u8>, Error> {
        let bytes = self.get(sub_url).await?.bytes().await;
        Ok(bytes
            .map_err(|e| Error::BlockchainError(e.to_string()))?
            .into_iter()
            .collect::<Vec<_>>())
    }

    // gets all the utxos and txs and height of chain, and returns the balance of the address
    pub async fn refresh_chain_data(&self, address: String) -> Result<(), Error> {
        let height = self
            .blockchain
            .get_height()
            .await
            .map_err(|e| Error::BlockchainError(e.to_string()))? as u64;
        //Don't need height anymore? even in wasm_wallet? can likely get rid

        match self.chain_data.lock() {
            Err(e) => {
                return Err(Error::BlockchainError(format!(
                    "Error unwrapping mutex for chain_data: {}",
                    e
                )))
            }
            Ok(chain_data) => {
                *chain_data.height.borrow_mut() = Some(height);
            }
        };

        debug!("fetching utxos from chain for address {}", address);

        //This only grabs the utxos for this one address. For the BDk we would need more than that. But for the
        //wasm wallet i guess that's fine? if it just has the one address for the DLCs? If router wallet doesn't
        //lock funds anymore, than this doesnt matter for it at all. Yeah that's right, the get_transaction function
        //could be left unimplemented for the router wallet because it would never get called in the party params function!
        //wow, so the big refactor isn't needed anyway!
        let fetched_utxos: Vec<UtxoResp> = self
            .get_from_json(&format!("address/{address}/utxo"))
            .await?;

        debug!("got {} utxos", fetched_utxos.len());

        let address = Address::from_str(&address).map_err(|e| {
            Error::BlockchainError(format!("Invalid Address format: {}", e.to_string()))
        })?;
        let new_utxos = fetched_utxos
            .into_iter()
            .map(|x| match x.txid.parse::<bitcoin::Txid>() {
                Ok(id) => Ok(Utxo {
                    address: address.clone(),
                    outpoint: OutPoint {
                        txid: id,
                        vout: x.vout,
                    },
                    redeem_script: Script::default(),
                    reserved: false,
                    tx_out: TxOut {
                        value: x.value,
                        script_pubkey: address.script_pubkey(),
                    },
                }),
                Err(e) => {
                    warn!("Error parsing bitcoin tx: {} - {}", x.txid, e.to_string());
                    Err(e)
                }
            })
            .filter_map(Result::ok)
            .collect::<Vec<Utxo>>();

        // let mut utxo_spent_pairs = Vec::new();
        // for utxo in utxos {
        //     let is_spent: UTXOSpent = self
        //         .get_from_json::<UTXOSpent>(&format!(
        //             "tx/{0}/outspend/{1}",
        //             &utxo.outpoint.txid, utxo.outpoint.vout
        //         ))
        //         .await
        //         .unwrap();
        //     utxo_spent_pairs.push((utxo, is_spent.spent));
        // }

        // probably don't need this part anymore, because we don't need to store the utxos, just the tx infos
        // ah yes we do, js_interface_wallet uses them in the get_utxos_for_amount function

        // Using a block closure here ensures the mutex is dropped at the end of the block
        {
            let cdata = self.chain_data.lock().map_err(|e| {
                Error::BlockchainError(format!("Error getting lock on mutex for chain_data: {}", e))
            })?;
            let mut stored_utxos = cdata.utxos.try_borrow_mut().map_err(|e| {
                Error::BlockchainError(format!("Error getting lock on mutex for chain_data: {}", e))
            })?;
            let stored_utxos = match stored_utxos.as_mut() {
                Some(utxos) => utxos,
                None => {
                    return Err(Error::BlockchainError(
                        "error getting chain_data utxos as a mut reference".to_string(),
                    ))
                }
            };
            stored_utxos.clear();
            stored_utxos.extend(new_utxos.clone());
        };

        debug!("fetching raw txs from chain");
        for utxo in new_utxos {
            let txid = utxo.outpoint.txid.to_string();
            trace!("fetching tx {}", txid);
            let raw_tx = self.get_bytes(&format!("tx/{}/raw", txid)).await?;
            let tx = Transaction::consensus_decode(&mut std::io::Cursor::new(&*raw_tx))
                .map_err(|e| Error::BlockchainError(e.to_string()))?;

            let local_cdata = self.chain_data.lock().map_err(|e| {
                Error::BlockchainError(format!("Error getting lock on mutex for chain_data: {}", e))
            })?;
            match local_cdata.txs.borrow_mut().as_mut() {
                Some(txs) => {
                    txs.insert(txid, tx.clone());
                }
                None => {
                    return Err(Error::BlockchainError(
                        "Unable to borrow the txs of the chain_data mutably".to_string(),
                    ))
                }
            };
        }
        Ok(())
    }

    pub fn get_utxos(&self) -> Result<Vec<Utxo>, Error> {
        let cdata = self.chain_data.lock().map_err(|e| {
            Error::BlockchainError(format!("Error getting lock on mutex for chain_data: {}", e))
        })?;
        let utxos_result = match cdata.utxos.borrow().as_ref() {
            Some(utxos) => Ok(utxos.clone()),
            None => Err(Error::BlockchainError(
                "Unable to borrow the txs of the chain_data mutably".to_string(),
            )),
        };
        utxos_result
    }

    pub async fn get_balance(&self) -> Result<u64, Error> {
        Ok(self.get_utxos()?.iter().map(|x| x.tx_out.value).sum())
    }
}

impl AsyncBlockchain for EsploraAsyncBlockchainProviderJsWallet {
    async fn get_transaction_confirmations_async(&self, tx_id: &Txid) -> Result<u32, Error> {
        let tx_status = self
            .get_from_json::<TxStatus>(&format!("tx/{tx_id}/status"))
            .await?;
        if tx_status.confirmed {
            let block_chain_height = self.blockchain.get_height().await.map_err(|e| {
                Error::BlockchainError(format!("Error getting blockchain height: {}", e))
            })? as u64;
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

impl Blockchain for EsploraAsyncBlockchainProviderJsWallet {
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
    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        let cdata = self.chain_data.lock().map_err(|e| {
            Error::BlockchainError(format!(
                "Error getting lock on mutex for chain_data: {}",
                e.to_string()
            ))
        })?;
        let txs = cdata.txs.borrow();
        let raw_txs = match txs.as_ref() {
            Some(txs) => Ok(txs),
            None => Err(Error::BlockchainError(
                "Unable to borrow the txs of the chain_data".to_string(),
            )),
        };
        match raw_txs?.get(&tx_id.to_string()) {
            Some(x) => Ok(x.clone()),
            None => Err(Error::BlockchainError(format!("tx not found {}", tx_id))),
        }
    }

    fn get_transaction_confirmations(&self, _tx_id: &Txid) -> Result<u32, Error> {
        // This is no longer used anywhere, as all calls can be the async version
        unimplemented!("use async version");
    }
}

impl WalletBlockchainProvider for EsploraAsyncBlockchainProviderJsWallet {
    fn get_utxos_for_address(&self, _address: &bitcoin::Address) -> Result<Vec<Utxo>, Error> {
        self.get_utxos()
    }

    fn is_output_spent(&self, txid: &Txid, vout: u32) -> Result<bool, Error> {
        let utxos = self.get_utxos()?;
        match utxos.into_iter().find(|utxo| utxo.outpoint.txid == *txid) {
            Some(utxo) => Ok(utxo.outpoint.vout == vout),
            None => Ok(false),
        }
    }
}

impl FeeEstimator for EsploraAsyncBlockchainProviderJsWallet {
    fn get_est_sat_per_1000_weight(
        &self,
        _confirmation_target: lightning::chain::chaininterface::ConfirmationTarget,
    ) -> u32 {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::*;

    fn get_esplora_provider_for_js_wallet() -> EsploraAsyncBlockchainProviderJsWallet {
        EsploraAsyncBlockchainProviderJsWallet::new(
            "esplora_url".to_string(),
            bitcoin::Network::Regtest,
        )
    }

    #[test]
    fn can_get_new_esplora_provider() {
        let provider = get_esplora_provider_for_js_wallet();
        assert_eq!("esplora_url", provider.host);
        assert_eq!(bitcoin::Network::Regtest, provider.network);
        let chain_data = provider
            .chain_data
            .lock()
            .expect("To be able to unwrap the mutex");
        assert_eq!(
            &ChainCacheData {
                utxos: Some(vec![]).into(),
                txs: Some(HashMap::new()).into(),
                height: Some(0).into(),
            },
            chain_data.deref()
        );
    }
}
