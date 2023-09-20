use bitcoin::Address;
use dlc_manager::{error::Error, Signer, Utxo, Wallet};
use secp256k1_zkp::{All, PublicKey, Secp256k1, SecretKey};

type Result<T> = core::result::Result<T, Error>;

pub struct DlcWallet {
    pub address: Address,
    seckey: SecretKey,
    secp_ctx: Secp256k1<All>,
}

impl DlcWallet {
    /// Create a new wallet instance.
    pub fn new(address: Address, seckey: SecretKey) -> Self {
        Self {
            address,
            seckey,
            secp_ctx: Secp256k1::new(),
        }
    }
}

impl Signer for DlcWallet {
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

impl Wallet for DlcWallet {
    fn get_new_address(&self) -> Result<Address> {
        Ok(self.address.clone())
    }

    fn get_new_secret_key(&self) -> Result<SecretKey> {
        Ok(self.seckey)
    }

    fn get_utxos_for_amount(
        &self,
        _amount: u64,
        _fee_rate: Option<u64>,
        _lock_utxos: bool,
    ) -> Result<Vec<Utxo>> {
        unimplemented!("Router wallet does not use UTXOs actively")
    }

    fn import_address(&self, _: &Address) -> Result<()> {
        Ok(())
    }
}
