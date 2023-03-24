use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bitcoin::consensus::Decodable;
use bitcoin::{
    Block, BlockHash, BlockHeader, Network, OutPoint, Script, Transaction, TxMerkleNode, TxOut,
    Txid,
};
use bitcoin_test_utils::tx_to_string;
use chrono::DateTime;
use dlc_manager::{error::Error, Blockchain, Utxo};
use lightning::chain::chaininterface::{BroadcasterInterface, ConfirmationTarget, FeeEstimator};
use reqwest::blocking::Response;
use serde::Deserialize;
use serde::Serialize;

const MIN_FEERATE: u32 = 253;

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum Target {
    Background,
    Normal,
    HighPriority,
}

pub struct BlockcypherBlockchainProvider {
    host: String,
    client: reqwest::blocking::Client,
    // async_client: reqwest::Client,
    network: Network,
    fees: Arc<HashMap<Target, AtomicU32>>,
}

impl BlockcypherBlockchainProvider {
    pub fn new(host: String, network: Network) -> Self {
        let host = format_host_url_with_network_path(host, network);
        let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
        fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
        fees.insert(Target::Normal, AtomicU32::new(2000));
        fees.insert(Target::HighPriority, AtomicU32::new(5000));
        let fees = Arc::new(fees);
        poll_for_fee_estimates(fees.clone(), &host);
        Self {
            host,
            network,
            client: reqwest::blocking::Client::new(),
            // async_client: reqwest::Client::new(),
            fees,
        }
    }

    fn get(&self, sub_url: &str) -> Result<Response, Error> {
        self.client
            .get(format!("{}{}", self.host, sub_url))
            .send()
            .map_err(|x| {
                dlc_manager::error::Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    x,
                ))
            })
    }

    // async fn get_async(&self, sub_url: &str) -> Result<reqwest::Response, reqwest::Error> {
    //     self.async_client
    //         .get(format!("{}{}", self.host, sub_url))
    //         .send()
    //         .await
    // }

    // fn get_text(&self, sub_url: &str) -> Result<String, Error> {
    //     self.get(sub_url)?.text().map_err(|x| {
    //         dlc_manager::error::Error::IOError(std::io::Error::new(std::io::ErrorKind::Other, x))
    //     })
    // }

    // fn get_u64(&self, sub_url: &str) -> Result<u64, Error> {
    //     self.get_text(sub_url)?
    //         .parse()
    //         .map_err(|_| Error::BlockchainError)
    // }

    // fn get_bytes(&self, sub_url: &str) -> Result<Vec<u8>, Error> {
    //     let bytes = self.get(sub_url)?.bytes();
    //     Ok(bytes
    //         .map_err(|_| Error::BlockchainError)?
    //         .into_iter()
    //         .collect::<Vec<_>>())
    // }

    fn get_from_json<T>(&self, sub_url: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get(sub_url)?
            .json::<T>()
            .map_err(|e| Error::OracleError(e.to_string()))
    }
}

impl Blockchain for BlockcypherBlockchainProvider {
    fn send_transaction(&self, transaction: &Transaction) -> Result<(), dlc_manager::error::Error> {
        let res = self
            .client
            .post(format!("{}tx", self.host))
            .body(tx_to_string(transaction))
            .send()
            .map_err(|x| {
                dlc_manager::error::Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    x,
                ))
            })?;
        if let Err(error) = res.error_for_status_ref() {
            let body = res.text().unwrap_or_default();
            return Err(dlc_manager::error::Error::InvalidParameters(format!(
                "Server returned error: {error} {body}"
            )));
        }
        Ok(())
    }

    fn get_network(
        &self,
    ) -> Result<bitcoin::network::constants::Network, dlc_manager::error::Error> {
        Ok(self.network)
    }

    fn get_blockchain_height(&self) -> Result<u64, dlc_manager::error::Error> {
        let block_chain_info: BlockchainInfo = self.get_from_json(&block_path())?;
        Ok(block_chain_info.height)
    }

    fn get_block_at_height(&self, height: u64) -> Result<Block, dlc_manager::error::Error> {
        let block = self.get_from_json::<BlockcypherBlockInfo>(&block_height_path(height));
        if block.is_ok() {
            let block = block.unwrap();
            let time = DateTime::parse_from_rfc3339(&block.time)
                .unwrap()
                .timestamp() as u32;
            let raw_txs = block
                .txids
                .iter()
                .map(|id| {
                    self.get_transaction(&id.parse::<Txid>().unwrap()) // Todo: better handle unwrap
                        .unwrap()
                })
                .collect();

            return Ok(Block {
                header: BlockHeader {
                    version: block.ver,
                    prev_blockhash: block.prev_block,
                    merkle_root: block.mrkl_root,
                    time,
                    bits: block.bits,
                    nonce: block.nonce,
                },
                txdata: raw_txs,
            });
        } else {
            return Err(block.err().unwrap());
        };
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, dlc_manager::error::Error> {
        let _tx_id_str = tx_id.to_string();
        let tx_json: BlockcypherTxInfo =
            self.get_from_json(&tx_path_from_str(&tx_id.to_string()))?;
        let binary_tx: Vec<u8> = hex::decode(tx_json.hex).unwrap();
        Transaction::consensus_decode(&mut std::io::Cursor::new(&*binary_tx))
            .map_err(|e| Error::StorageError(e.to_string()))
    }

    fn get_transaction_confirmations(
        &self,
        tx_id: &Txid,
    ) -> Result<u32, dlc_manager::error::Error> {
        let _tx_id_str = tx_id.to_string();
        let tx_json: BlockcypherTxInfo =
            self.get_from_json(&tx_path_from_str(&tx_id.to_string()))?;
        Ok(tx_json.confirmations)
    }
}

impl simple_wallet::WalletBlockchainProvider for BlockcypherBlockchainProvider {
    fn get_utxos_for_address(&self, address: &bitcoin::Address) -> Result<Vec<Utxo>, Error> {
        let address_info: BlockcypherAddress =
            self.get_from_json(&format!("addrs/{address}&unspentOnly=true"))?;

        let utxos: Vec<UtxoResp> = address_info.txrefs.unwrap_or(vec![]);
        utxos
            .into_iter()
            .map(|x| {
                Ok(Utxo {
                    address: address.clone(),
                    outpoint: OutPoint {
                        txid: x.tx_hash.parse().map_err(|_| Error::BlockchainError)?,
                        vout: x.tx_output_n,
                    },
                    redeem_script: Script::default(),
                    reserved: false,
                    tx_out: TxOut {
                        value: x.value,
                        script_pubkey: address.script_pubkey(),
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()
    }

    fn is_output_spent(&self, txid: &Txid, vout: u32) -> Result<bool, Error> {
        let tx_json: BlockcypherTxInfo =
            self.get_from_json(&tx_path_from_str(&txid.to_string()))?;
        let outputs = tx_json.outputs;
        if vout > outputs.len().try_into().unwrap() {
            return Err(Error::OracleError("Invalid vout index for tx".to_string()));
        }
        let out = &outputs[vout as usize];
        Ok(out.spent_by.is_some())
    }
}

impl FeeEstimator for BlockcypherBlockchainProvider {
    fn get_est_sat_per_1000_weight(&self, confirmation_target: ConfirmationTarget) -> u32 {
        let est = match confirmation_target {
            ConfirmationTarget::Background => self
                .fees
                .get(&Target::Background)
                .unwrap()
                .load(Ordering::Acquire),
            ConfirmationTarget::Normal => self
                .fees
                .get(&Target::Normal)
                .unwrap()
                .load(Ordering::Acquire),
            ConfirmationTarget::HighPriority => self
                .fees
                .get(&Target::HighPriority)
                .unwrap()
                .load(Ordering::Acquire),
        };
        u32::max(est, MIN_FEERATE)
    }
}

// impl BlockSource for BlockcypherBlockchainProvider {
//     fn get_header<'a>(
//         &'a self,
//         header_hash: &'a bitcoin::BlockHash,
//         _: Option<u32>,
//     ) -> lightning_block_sync::AsyncBlockSourceResult<'a, lightning_block_sync::BlockHeaderData>
//     {
//         Box::pin(async move {
//             let block_info: BlockInfo = self
//                 .get_async(&format!("block/{header_hash:x}"))
//                 .await
//                 .map_err(BlockSourceError::transient)?
//                 .json()
//                 .await
//                 .map_err(BlockSourceError::transient)?;
//             let header_hex_str = self
//                 .get_async(&format!("block/{header_hash:x}/header"))
//                 .await
//                 .map_err(BlockSourceError::transient)?
//                 .text()
//                 .await
//                 .map_err(BlockSourceError::transient)?;
//             let header_hex = bitcoin_test_utils::str_to_hex(&header_hex_str);
//             let header = BlockHeader::consensus_decode(&mut std::io::Cursor::new(&*header_hex))
//                 .expect("to have a valid header");
//             header.validate_pow(&header.target()).unwrap();
//             Ok(BlockHeaderData {
//                 header,
//                 height: block_info.height,
//                 // Blockcypher doesn't seem to make this available.
//                 chainwork: Uint256::from_u64(10).unwrap(),
//             })
//         })
//     }

//     fn get_block<'a>(
//         &'a self,
//         header_hash: &'a bitcoin::BlockHash,
//     ) -> lightning_block_sync::AsyncBlockSourceResult<'a, BlockData> {
//         Box::pin(async move {
//             let block_raw = self
//                 .get_async(&format!("block/{header_hash:x}/raw"))
//                 .await
//                 .map_err(BlockSourceError::transient)?
//                 .bytes()
//                 .await
//                 .map_err(BlockSourceError::transient)?;
//             let block = Block::consensus_decode(&mut std::io::Cursor::new(&*block_raw))
//                 .expect("to have a valid header");
//             Ok(BlockData::FullBlock(block))
//         })
//     }

//     fn get_best_block(
//         &self,
//     ) -> lightning_block_sync::AsyncBlockSourceResult<(bitcoin::BlockHash, Option<u32>)> {
//         Box::pin(async move {
//             let block_tip_hash: String = self
//                 .get_async("blocks/tip/hash")
//                 .await
//                 .map_err(BlockSourceError::transient)?
//                 .text()
//                 .await
//                 .map_err(BlockSourceError::transient)?;
//             let block_tip_height: u32 = self
//                 .get_async("blocks/tip/height")
//                 .await
//                 .map_err(BlockSourceError::transient)?
//                 .text()
//                 .await
//                 .map_err(BlockSourceError::transient)?
//                 .parse()
//                 .map_err(BlockSourceError::transient)?;
//             Ok((
//                 BlockHash::from_hex(&block_tip_hash).map_err(BlockSourceError::transient)?,
//                 Some(block_tip_height),
//             ))
//         })
//     }
// }

impl BroadcasterInterface for BlockcypherBlockchainProvider {
    fn broadcast_transaction(&self, tx: &Transaction) {
        let client = self.client.clone();
        let host = self.host.clone();
        let body = format!("{{\"tx\":\"{}\"}}", bitcoin_test_utils::tx_to_string(tx));
        std::thread::spawn(move || {
            match client.post(format!("{host}txs/push")).body(body).send() {
                Err(_) => {}
                Ok(res) => {
                    if res.error_for_status_ref().is_err() {
                        // let body = res.text().unwrap_or_default();
                        // TODO(tibo): log
                    }
                }
            };
        });
    }
}

// #[derive(Serialize, Deserialize, Debug)]
// struct TxStatus {
//     confirmed: bool,
//     block_height: Option<u64>,
//     block_hash: Option<String>,
// }

#[derive(Serialize, Deserialize, Debug)]
struct UtxoResp {
    tx_hash: String,
    tx_output_n: u32,
    value: u64,
    // status: UtxoStatus,
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(untagged)]
// pub enum UtxoStatus {
//     Confirmed {
//         confirmed: bool,
//         block_height: u64,
//         block_hash: String,
//         block_time: u64,
//     },
//     Unconfirmed {
//         confirmed: bool,
//     },
// }

#[derive(Serialize, Deserialize, Debug)]
struct SpentResp {
    spent: bool,
}

// API Paths
fn block_path() -> String {
    "".to_string()
}

fn block_height_path(height: u64) -> String {
    format!("blocks/{height}")
}

fn tx_path_from_str(tx_id: &str) -> String {
    format!("txs/{tx_id}?includeHex=true")
}

fn store_estimate_for_target(
    fees: &Arc<HashMap<Target, AtomicU32>>,
    fee_estimates: &BlockchainInfo,
    target: Target,
) {
    #[allow(clippy::redundant_clone)]
    let val = get_estimate_for_target(fee_estimates, target.clone());
    fees.get(&target)
        .unwrap()
        .store(val, std::sync::atomic::Ordering::Relaxed);
}

fn get_estimate_for_target(fee_estimates: &BlockchainInfo, target: Target) -> u32 {
    match target {
        Target::Background => fee_estimates.low_fee_per_kb,
        Target::Normal => fee_estimates.medium_fee_per_kb,
        Target::HighPriority => fee_estimates.high_fee_per_kb,
    }
}

fn format_host_url_with_network_path(host: String, network: Network) -> String {
    match host.chars().last() {
        Some('/') => format!("{}{}", host, get_network_url(network)),
        _ => format!("{}/{}", host, get_network_url(network)),
    }
}

fn get_network_url(network: Network) -> String {
    let network_url: &str = match network {
        Network::Testnet => "v1/btc/test3/",
        Network::Bitcoin => "v1/btc/main/",
        Network::Regtest => "v1/bcy/test/",
        Network::Signet => todo!(),
    };
    return network_url.to_string();
}

// This has been updated for Blockcypher
fn poll_for_fee_estimates(fees: Arc<HashMap<Target, AtomicU32>>, host: &str) {
    let host = host.to_owned();
    std::thread::spawn(move || loop {
        if let Ok(res) = reqwest::blocking::get(format!("{host}")) {
            if let Ok(blockchain_info) = res.json::<BlockchainInfo>() {
                store_estimate_for_target(&fees, &blockchain_info, Target::Background);
                store_estimate_for_target(&fees, &blockchain_info, Target::HighPriority);
                store_estimate_for_target(&fees, &blockchain_info, Target::Normal);
            }
        }

        std::thread::sleep(Duration::from_secs(60));
    });
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockcypherAddress {
    txrefs: Option<Vec<UtxoResp>>,
}
#[derive(Serialize, Deserialize, Debug)]
struct BlockcypherBlockInfo {
    hash: String,
    height: u64,
    chain: String,
    total: u64,
    fees: u32,
    size: u32,
    ver: i32,
    time: String,
    received_time: String,
    coinbase_addr: Option<String>,
    relayed_by: String,
    bits: u32,
    nonce: u32,
    n_tx: u32,
    prev_block: BlockHash,
    mrkl_root: TxMerkleNode,
    txids: Vec<String>,
    depth: u32,
    prev_block_url: String,
    tx_url: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct BlockcypherTxInfo {
    hash: String,
    hex: String,
    confirmations: u32,
    outputs: Vec<BlockcypherOutput>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockcypherOutput {
    spent_by: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockchainInfo {
    name: String,
    height: u64,
    high_fee_per_kb: u32,
    medium_fee_per_kb: u32,
    low_fee_per_kb: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockInfo {
    height: u32,
}

// fn sats_per_vbyte_to_sats_per_1000_weight(input: f32) -> u32 {
//     (input * 1000.0 / 4.0).round() as u32
// }

// #[derive(Serialize, Deserialize, Debug)]
// #[serde(untagged)]
// pub enum OutSpendResp {
//     Spent(OutSpendInfo),
//     Unspent { spent: bool },
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct OutSpendInfo {
//     pub spent: bool,
//     pub txid: Txid,
//     pub vin: usize,
//     pub status: UtxoStatus,
// }

#[cfg(test)]
mod tests {
    extern crate mockito;
    use bitcoin::{consensus::deserialize, hashes::hex::FromHex};

    use self::mockito::{mock, Mock};
    use super::*;

    fn get_block_height_mock(path: &str) -> Mock {
        println!("Mocking at path {path}");
        mock("GET", path)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "hash": "00000000480ad525116799dd332495b82dfa93ef1cffb7921e054a479f1d79b3",
                "height": 2424237,
                "chain": "BTC.test3",
                "total": 0,
                "fees": 0,
                "size": 176,
                "vsize": 176,
                "ver": 536870912,
                "time": "2023-03-13T15:43:27Z",
                "received_time": "2023-03-13T15:27:14.595Z",
                "relayed_by": "44.205.255.132:18333",
                "bits": 486604799,
                "nonce": 194132897,
                "n_tx": 1,
                "prev_block": "0000000000000002a2fc13454f76e1d00afe100ce843511b85084194e8c7f859",
                "mrkl_root": "fb4bff2978ffc6ae0d256195190559d8542d8d9a0c020e2d631c1bf4893bffe3",
                "txids": [
                    "fb4bff2978ffc6ae0d256195190559d8542d8d9a0c020e2d631c1bf4893bffe3"
                ],
                "depth": 7,
                "prev_block_url": "https://api.blockcypher.com/v1/btc/test3/blocks/0000000000000002a2fc13454f76e1d00afe100ce843511b85084194e8c7f859",
                "tx_url": "https://api.blockcypher.com/v1/btc/test3/txs/"
            }"#)
            .create()
    }

    fn get_raw_tx_mock(path: &str) -> Mock {
        mock("GET", path)
            .with_status(200)
            .with_body(r#"{
                "hash": "fb4bff2978ffc6ae0d256195190559d8542d8d9a0c020e2d631c1bf4893bffe3",
                "hex": "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0d03adfd24084ee26f68987880f0ffffffff01be402500000000001600146370e621face1d613380d321325c05de859e1b4a0000000001000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0d03adfd24084ee26f68987880f0ffffffff01be402500000000001600146370e621face1d613380d321325c05de859e1b4a00000000",
                "confirmations": 0,
                "outputs": [
                    {
                        "value": 2441406,
                        "script": "00146370e621face1d613380d321325c05de859e1b4a",
                        "addresses": [
                            "bc1qvdcwvg06ecwkzvuq6vsnyhq9m6zeux628gjxjp"
                        ],
                        "script_type": "pay-to-witness-pubkey-hash"
                    }
                ]
            }"#)
            .create()
    }

    #[test]
    fn get_block_height_test() {
        let network = bitcoin::Network::Testnet;
        let url = &mockito::server_url();

        // test_block_hash = "00000000480ad525116799dd332495b82dfa93ef1cffb7921e054a479f1d79b3";
        let block_hex = Vec::from_hex("0000002059f8c7e8944108851b5143e80c10fe0ad0e1764f4513fca20200000000000000e3ff3b89f41b1c632d0e020c9a8d2d54d85905199561250daec6ff7829ff4bfb9f440f64ffff001da13b920b0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0d03adfd24084ee26f68987880f0ffffffff01be402500000000001600146370e621face1d613380d321325c05de859e1b4a00000000").unwrap();
        let expected_block: Block = deserialize(&block_hex).unwrap();

        let _raw_tx_mock = get_raw_tx_mock(&format!(
            "/{}{}",
            get_network_url(network),
            tx_path_from_str("fb4bff2978ffc6ae0d256195190559d8542d8d9a0c020e2d631c1bf4893bffe3")
        ));

        let _block_mock = get_block_height_mock(&format!(
            "/{}{}",
            get_network_url(network),
            block_height_path(2424237)
        ));

        let blockcypher = Arc::new(BlockcypherBlockchainProvider::new(
            url.to_string(),
            bitcoin::Network::Testnet,
        ));

        let received_block = blockcypher.get_block_at_height(2424237).unwrap();

        assert_eq!(expected_block, received_block);
    }
}
