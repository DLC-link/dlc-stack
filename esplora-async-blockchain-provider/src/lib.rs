use bdk::esplora_client::TxStatus;
use bdk::esplora_client::{AsyncClient, Builder};
use bitcoin::consensus::Decodable;
use bitcoin::{Address, Block, Network, OutPoint, Script, Transaction, TxOut, Txid};
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
use wasm_bindgen_futures::spawn_local;

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

pub struct EsploraAsyncBlockchainProvider {
    host: String,
    pub blockchain: EsploraBlockchain,
    chain_data: Arc<Mutex<ChainCacheData>>,
    network: Network,
}

pub struct TxRawWithConf {
    pub raw_tx: Vec<u8>,
    pub confirmations: u32,
}
struct ChainCacheData {
    utxos: RefCell<Option<Vec<Utxo>>>,
    txs: RefCell<Option<HashMap<String, TxRawWithConf>>>,
    height: RefCell<Option<u64>>,
}

impl EsploraAsyncBlockchainProvider {
    pub fn new(host: String, network: Network) -> Self {
        let client_builder = Builder::new(&host).timeout(REQWEST_TIMEOUT);
        let url_client = AsyncClient::from_builder(client_builder).unwrap();
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

    // async fn get_text(&self, sub_url: &str) -> Result<String, Error> {
    //     self.get(sub_url).await.unwrap().text().await.map_err(|x| {
    //         dlc_manager::error::Error::IOError(std::io::Error::new(std::io::ErrorKind::Other, x))
    //     })
    // }

    // async fn get_u64(&self, sub_url: &str) -> Result<u64, Error> {
    //     self.get_text(sub_url)
    //         .await
    //         .unwrap()
    //         .parse()
    //         .map_err(|e: std::num::ParseIntError| Error::BlockchainError(e.to_string()))
    // }

    // gets all the utxos and txs and height of chain, and returns the balance of the address
    pub async fn refresh_chain_data(&self, address: String) -> Result<(), Error> {
        let height = self.blockchain.get_height().await.unwrap() as u64;

        *self.chain_data.lock().unwrap().height.borrow_mut() = Some(height);

        debug!("fetching utxos from chain for address {}", address);

        let utxos: Vec<UtxoResp> = self
            .get_from_json(&format!("address/{address}/utxo"))
            .await
            .unwrap();

        debug!("got {} utxos", utxos.len());

        let address = Address::from_str(&address).unwrap();
        let mut utxos = utxos
            .into_iter()
            .map(|x| Utxo {
                address: address.clone(),
                outpoint: OutPoint {
                    txid: x
                        .txid
                        .parse()
                        .map_err(|e: <bitcoin::Txid as FromStr>::Err| {
                            Error::BlockchainError(e.to_string())
                        })
                        .unwrap(),
                    vout: x.vout,
                },
                redeem_script: Script::default(),
                reserved: false,
                tx_out: TxOut {
                    value: x.value,
                    script_pubkey: address.script_pubkey(),
                },
            })
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

        self.chain_data
            .lock()
            .unwrap()
            .utxos
            .try_borrow_mut()
            .unwrap() // FIXME this blows up sometimes!
            .as_mut()
            .unwrap()
            .clear();
        self.chain_data
            .lock()
            .unwrap()
            .utxos
            .borrow_mut()
            .as_mut()
            .unwrap()
            .append(&mut utxos);

        debug!("fetching raw txs from chain");

        let chain_data = self.chain_data.lock().unwrap();

        for utxo in chain_data.utxos.borrow().as_ref().unwrap() {
            let txid = utxo.outpoint.txid.to_string();
            trace!("fetching tx {}", txid);
            let tx: Vec<u8> = self.get_bytes(&format!("tx/{}/raw", txid)).await?;

            let tx_status = self
                .get_from_json::<TxStatus>(&format!("tx/{txid}/status"))
                .await?;
            let confirmations = match tx_status.confirmed {
                true => {
                    if let Some(block_height) = tx_status.block_height {
                        (height - block_height as u64 + 1) as u32
                    } else {
                        warn!("tx {} has no block height", txid);
                        0
                    }
                }
                false => 0,
            };

            chain_data.txs.borrow_mut().as_mut().unwrap().insert(
                txid,
                TxRawWithConf {
                    raw_tx: tx.clone(),
                    confirmations, // set this for real
                },
            );
        }
        Ok(())
    }

    pub fn get_utxos(&self) -> Result<Vec<Utxo>, Error> {
        Ok(self
            .chain_data
            .lock()
            .unwrap()
            .utxos
            .borrow()
            .as_ref()
            .unwrap()
            .clone())
    }

    pub async fn get_balance(&self) -> Result<u64, Error> {
        Ok(self
            .chain_data
            .lock()
            .unwrap()
            .utxos
            .borrow()
            .as_ref()
            .unwrap()
            .iter()
            .map(|x| x.tx_out.value)
            .sum())
    }
}

impl Blockchain for EsploraAsyncBlockchainProvider {
    fn send_transaction(&self, transaction: &Transaction) -> Result<(), Error> {
        let x = self.blockchain.clone();
        let y = transaction.clone();
        spawn_local(async move {
            match x.broadcast(&y).await {
                Ok(_) => (),
                Err(e) => error!("Error broadcasting tx: {}", e),
            }
        });
        Ok(())
    }

    fn get_network(&self) -> Result<bitcoin::network::constants::Network, Error> {
        Ok(self.network)
    }

    fn get_blockchain_height(&self) -> Result<u64, Error> {
        Ok(self.chain_data.lock().unwrap().height.borrow().unwrap())
    }

    fn get_block_at_height(&self, _height: u64) -> Result<Block, Error> {
        //only used for lightning
        unimplemented!();
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        let chain_data = self.chain_data.lock().unwrap();
        let txs = chain_data.txs.borrow();
        let raw_txs = txs.as_ref().unwrap();
        let raw_tx = match raw_txs.get(&tx_id.to_string()) {
            Some(x) => x.raw_tx.clone(),
            None => return Err(Error::BlockchainError(format!("tx not found {}", tx_id))),
        };
        Transaction::consensus_decode(&mut std::io::Cursor::new(&*raw_tx))
            .map_err(|e| Error::BlockchainError(e.to_string()))
    }

    fn get_transaction_confirmations(&self, tx_id: &Txid) -> Result<u32, Error> {
        let chain_data = self.chain_data.lock().unwrap();
        let txs = chain_data.txs.borrow();
        let raw_txs = txs.as_ref().unwrap();
        let confirmations = match raw_txs.get(&tx_id.to_string()) {
            Some(x) => x.confirmations,
            None => return Err(Error::BlockchainError(format!("tx not found {}", tx_id))),
        };
        Ok(confirmations)
    }
}

impl WalletBlockchainProvider for EsploraAsyncBlockchainProvider {
    fn get_utxos_for_address(&self, _address: &bitcoin::Address) -> Result<Vec<Utxo>, Error> {
        Ok(self
            .chain_data
            .lock()
            .unwrap()
            .utxos
            .borrow()
            .as_ref()
            .unwrap()
            .clone())
    }

    fn is_output_spent(&self, txid: &Txid, vout: u32) -> Result<bool, Error> {
        let utxos = self
            .chain_data
            .lock()
            .unwrap()
            .utxos
            .borrow()
            .as_ref()
            .unwrap()
            .clone();
        let matched_utxo = utxos.into_iter().find(|utxo| utxo.outpoint.txid == *txid);
        if matched_utxo.is_none() {
            return Ok(false);
        }
        let matched_utxo = matched_utxo.unwrap();
        Ok(matched_utxo.outpoint.vout == vout)
    }
}

impl FeeEstimator for EsploraAsyncBlockchainProvider {
    fn get_est_sat_per_1000_weight(
        &self,
        _confirmation_target: lightning::chain::chaininterface::ConfirmationTarget,
    ) -> u32 {
        unimplemented!()
    }
}
