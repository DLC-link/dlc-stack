use parking_lot::Mutex;
use std::{ops::Deref, sync::Arc};

use bdk::{
    database::AnyDatabase,
    sled,
    wallet::coin_selection::{BranchAndBoundCoinSelection, CoinSelectionAlgorithm},
    FeeRate, WeightedUtxo,
};
use bitcoin::{hashes::Hash, Address, Network, Script};
use dlc_manager::{error::Error, Signer, Utxo, Wallet};
use log::debug;
use secp256k1_zkp::{All, PublicKey, Secp256k1, SecretKey};

type Result<T> = core::result::Result<T, Error>;

pub struct DlcBdkWallet {
    pub bdk_wallet: Arc<Mutex<bdk::Wallet<sled::Tree>>>,
    pub address: Address,
    seckey: SecretKey,
    secp_ctx: Secp256k1<All>,
    _network: Network,
}

impl DlcBdkWallet {
    /// Create a new wallet instance.
    pub fn new(
        bdk_wallet: Arc<Mutex<bdk::Wallet<sled::Tree>>>,
        address: Address,
        seckey: SecretKey,
        _network: Network,
    ) -> Self {
        Self {
            bdk_wallet,
            address,
            seckey,
            secp_ctx: Secp256k1::new(),
            _network,
        }
    }
}

impl Signer for DlcBdkWallet {
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

impl Wallet for DlcBdkWallet {
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
        debug!(
            "get_utxos_for_amount: amount: {} with fee_rate {:?}",
            amount, fee_rate
        );
        let dummy_pubkey: PublicKey =
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .parse()
                .unwrap();
        let dummy_drain =
            Script::new_v0_p2wpkh(&bitcoin::WPubkeyHash::hash(&dummy_pubkey.serialize()));

        let org_utxos = self
            .bdk_wallet
            .lock()
            // .unwrap()
            .list_unspent()
            .unwrap()
            .clone();
        let utxos = org_utxos
            .iter()
            .map(|x| WeightedUtxo {
                utxo: bdk::Utxo::Local(x.clone()),
                satisfaction_weight: 107,
            })
            .collect::<Vec<_>>();

        let selection = BranchAndBoundCoinSelection::default()
            .coin_select(
                &AnyDatabase::Sled(self.bdk_wallet.lock().database().deref().clone()),
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
                redeem_script: Script::new(), // What is this for, and where can I get it when using BDK to manage UTXOs?
                reserved: false,
            });
        }

        debug!("returning found utxos: {:?}", res);
        Ok(res)
    }

    fn import_address(&self, _: &Address) -> Result<()> {
        Ok(())
    }
}
