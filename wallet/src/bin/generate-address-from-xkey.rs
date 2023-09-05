use bdk::descriptor;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use std::{env, str::FromStr};

use serde_json::json;

fn main() {
    // Setup Blockchain Connection Object
    let network = match env::var("BITCOIN_NETWORK").as_deref() {
        Ok("bitcoin") => bitcoin::Network::Bitcoin,
        Ok("testnet") => bitcoin::Network::Testnet,
        Ok("signet") => bitcoin::Network::Signet,
        Ok("regtest") => bitcoin::Network::Regtest,
        _ => panic!(
            "Unknown Bitcoin Network, make sure to set BITCOIN_NETWORK in your env variables"
        ),
    };

    let xpriv_str = env::var("XPKEY").expect("XPKEY env variable not set");
    let xpriv = ExtendedPrivKey::from_str(&xpriv_str).expect("Unable to decode xpriv env variable");
    let secp = bitcoin::secp256k1::Secp256k1::new();

    let external_derivation_path =
        DerivationPath::from_str("m/44h/0h/0h/0").expect("A valid derivation path");

    let signing_external_descriptor = descriptor!(wpkh((
        xpriv,
        external_derivation_path.extend([ChildNumber::Normal { index: 0 }])
    )))
    .unwrap();

    let x = signing_external_descriptor.0.clone();

    let address = x.at_derivation_index(0).address(network).unwrap();
    let derived_ext_xpriv = xpriv
        .derive_priv(
            &secp,
            &external_derivation_path.extend([
                ChildNumber::Normal { index: 0 },
                ChildNumber::Normal { index: 0 },
            ]),
        )
        .unwrap();
    let pubkey = ExtendedPubKey::from_priv(&secp, &derived_ext_xpriv);
    println!(
        "{}",
        json!({ "public_key": pubkey, "network": network, "address": address })
    )
}
