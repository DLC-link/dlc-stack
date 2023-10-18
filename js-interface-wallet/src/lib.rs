#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]

use bdk::{
    database::{AnyDatabase, MemoryDatabase},
    wallet::coin_selection::{BranchAndBoundCoinSelection, CoinSelectionAlgorithm},
    FeeRate, KeychainKind, LocalUtxo, Utxo as BdkUtxo, WeightedUtxo,
};
use bitcoin::{hashes::Hash, Address, PrivateKey, Script, Txid};
use dlc_manager::{error::Error, Blockchain, Signer, Utxo, Wallet};
use lightning::chain::chaininterface::FeeEstimator;
use secp256k1_zkp::{All, PublicKey, Secp256k1, SecretKey};
use std::{str::FromStr, sync::RwLock};
type Result<T> = core::result::Result<T, Error>;

/// Trait providing blockchain information to the wallet.
pub trait WalletBlockchainProvider: Blockchain + FeeEstimator {
    fn get_utxos_for_address(&self, address: &Address) -> Result<Vec<Utxo>>;
    fn is_output_spent(&self, txid: &Txid, vout: u32) -> Result<bool>;
}

pub struct JSInterfaceWallet {
    address: Address,
    secp_ctx: Secp256k1<All>,
    seckey: SecretKey,
    utxos: RwLock<Vec<Utxo>>,
}

impl JSInterfaceWallet {
    pub fn new(address_str: String, privkey: PrivateKey) -> Self {
        Self {
            address: Address::from_str(&address_str).expect("A valid address for the DLC library"),
            secp_ctx: Secp256k1::new(),
            seckey: privkey.inner,
            utxos: RwLock::new(vec![]),
        }
    }

    pub fn set_utxos(&self, mut utxos: Vec<Utxo>) -> Result<()> {
        let mut utxos_ref = self.utxos.write().map_err(|_e| {
            Error::WalletError("Unable to get write lock for DLC JSInterfaceWallet UTXOs".into())
        })?;
        utxos_ref.clear();
        utxos_ref.append(&mut utxos);
        Ok(())
    }
}

impl Signer for JSInterfaceWallet {
    fn sign_tx_input(
        &self,
        tx: &mut bitcoin::Transaction,
        input_index: usize,
        tx_out: &bitcoin::TxOut,
        _: Option<bitcoin::Script>,
    ) -> Result<()> {
        dlc::util::sign_p2wpkh_input(
            &self.secp_ctx,
            &self.seckey,
            tx,
            input_index,
            bitcoin::EcdsaSighashType::All,
            tx_out.value,
        )?;
        Ok(())
    }

    fn get_secret_key_for_pubkey(&self, _pubkey: &PublicKey) -> Result<SecretKey> {
        Ok(self.seckey)
    }
}

impl Wallet for JSInterfaceWallet {
    fn get_new_address(&self) -> Result<Address> {
        Ok(self.address.clone())
    }

    fn get_new_secret_key(&self) -> Result<SecretKey> {
        Ok(self.seckey)
    }

    fn get_utxos_for_amount(
        &self,
        amount: u64,
        fee_rate: Option<u64>,
        _lock_utxos: bool,
    ) -> Result<Vec<Utxo>> {
        let dummy_pubkey: PublicKey =
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .parse()
                .expect("the static string to parse to a key");
        let dummy_drain =
            Script::new_v0_p2wpkh(&bitcoin::WPubkeyHash::hash(&dummy_pubkey.serialize()));
        let org_utxos = self
            .utxos
            .read()
            .map_err(|_e| {
                Error::WalletError(
                    "Unable to get write lock for DLC JSInterfaceWallet UTXOs".into(),
                )
            })?
            .clone();
        let utxos = org_utxos
            .iter()
            .filter(|x| !x.reserved)
            .map(|x| WeightedUtxo {
                utxo: BdkUtxo::Local(LocalUtxo {
                    outpoint: x.outpoint,
                    txout: x.tx_out.clone(),
                    keychain: KeychainKind::External,
                    is_spent: false,
                }),
                satisfaction_weight: 107,
            })
            .collect::<Vec<_>>();

        let selection = BranchAndBoundCoinSelection::default()
            .coin_select(
                &AnyDatabase::Memory(MemoryDatabase::new()),
                vec![],
                utxos,
                FeeRate::from_sat_per_vb(fee_rate.unwrap_or(0) as f32),
                amount,
                &dummy_drain,
            )
            .map_err(|x| Error::WalletError(Box::new(x)))?;

        let mut res = Vec::new();

        for utxo in selection.selected {
            res.push(dlc_manager::Utxo {
                outpoint: utxo.outpoint(),
                tx_out: utxo.txout().clone(),
                address: self.address.clone(),
                redeem_script: Script::new(),
                reserved: false,
            });
        }
        Ok(res)
    }

    fn import_address(&self, _: &Address) -> Result<()> {
        // unimplemented!()
        Ok(())
    }
}

#[derive(Clone)]
struct UtxoWrap {
    utxo: Utxo,
}

impl rust_bitcoin_coin_selection::Utxo for UtxoWrap {
    fn get_value(&self) -> u64 {
        self.utxo.tx_out.value
    }
}
