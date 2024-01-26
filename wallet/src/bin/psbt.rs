use bdk::descriptor;
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::{DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey};
// use bdk::miniscript::Segwitv0;
use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPubKey};
// use miniscript;
use miniscript::descriptor::DescriptorType;
use std::env;
use std::str::FromStr;

use serde_json::json;

use miniscript::bitcoin::secp256k1::{Secp256k1, Verification};
use miniscript::bitcoin::{Address, Network};
use miniscript::{DefiniteDescriptorKey, Descriptor, DescriptorPublicKey};

const XPUB_1: &str = "tpubDF6XCgok2eqEzkdpZeNT3V6EvBcxtahc2z5x8fZUWQ6nZWRnaMBmyXpPjeZKmSukZuSqUTPZG8nNMnDohq9X94AZPN8G1yGUp3RhpKXvnNk";
// const XPUB_1: &str = "xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB";
// const XPUB_1: &str = "03417129311ed34c242c012cd0a3e0b9bca0065f742d0dfb63c78083ea6a02d4d9";
const XPUB_2: &str = "tpubDEdNFYPVLj1kKWteKs621XJQkzSAPMDV2tiD5jAPHumVFAwgEqk6S2Brw7bxoDy4dZfiKwUvvTrKyNxq6vRM1KmwdbKme7Tny8AptBQs2cM";
// const XPUB_2: &str = "xpub69H7F5d8KSRgmmdJg2KhpAK8SR3DjMwAdkxj3ZuxV27CprR9LgpeyGmXUbC6wb7ERfvrnKZjXoUmmDznezpbZb7ap6r1D3tgFxHmwMkQTPH";
// const XPUB_2: &str = "032d672a1a91cc39d154d366cd231983661b0785c7f27bc338447565844f4a6813";
// const XPUB_3: &str = "027a3565454fe1b749bccaef22aff72843a9c3efefd7b16ac54537a0c23f0ec0de";

fn main() {
    // let s = format!("wsh(sortedmulti(1,{},{}))", XPUB_1, XPUB_2);
    // "and_v(v:pk({}),andor(pk({}),hash160({}),older(1008)))",
    //and_v(c:pk({}),andor(c:pk({}),hash160(64cb957b5cae09b6e0767caf1623d11dd1cc143e),older(1008)))\
    // let s = format!("thresh(2,pk_h({}),pk_h({}))", XPUB_1, XPUB_2);
    let s = format!("wsh(multi(2,{},{}))", XPUB_1, XPUB_2);
    // let s = format!("pk({})", XPUB_1);
    // let bridge_descriptor = Descriptor::from_str(&s).unwrap();
    let bridge_descriptor = match Descriptor::<DefiniteDescriptorKey>::from_str(&s) {
        Ok(d) => d,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    assert!(bridge_descriptor.sanity_check().is_ok());
    println!(
        "Bridge pubkey script: {}",
        bridge_descriptor.script_pubkey()
    );
    println!(
        "Bridge address: {}",
        match bridge_descriptor.address(Network::Regtest) {
            Ok(a) => a.to_string(),
            Err(e) => format!("Error: {}", e),
        }
    );
    println!(
        "Weight for witness satisfaction cost {}",
        bridge_descriptor.max_weight_to_satisfy().unwrap()
    );

    // Derive the P2SH address.
    // assert_eq!(
    //     desc.address(miniscript::bitcoin::Network::Bitcoin)
    //         .unwrap()
    //         .to_string(),
    //     "3CJxbQBfWAe1ZkKiGQNEYrioV73ZwvBWns"
    // );
    // let secp = Secp256k1::new();
    // let address = desc
    //     .derived_descriptor(&secp)
    //     .unwrap()
    //     .address(Network::Bitcoin)
    //     .unwrap();
    // println!("{}", address);

    // // Check whether the descriptor is safe. This checks whether all spend paths are accessible in
    // // the Bitcoin network. It may be possible that some of the spend paths require more than 100
    // // elements in Wsh scripts or they contain a combination of timelock and heightlock.
    // assert!(desc.sanity_check().is_ok());

    // Estimate the satisfaction cost.
    // scriptSig: OP_PUSH34 <OP_0 OP_32 <32-byte-hash>>
    // = (1 + 1 + 1 + 32) * 4 = 140 WU
    // redeemScript: varint <OP_33 <pk1> OP_CHECKSIG OP_IFDUP OP_NOTIF OP_33 <pk2> OP_CHECKSIG OP_ENDIF>
    // = 1 + (1 + 33 + 1 + 1 + 1 + 1 + 33 + 1 + 1) = 74 WU
    // stackItem[Sig]: varint <sig+sighash>
    // = 1 + 73 = 74 WU
    // Expected satisfaction weight: 140 + 74 + 74 = 288
    // assert_eq!(desc.max_weight_to_satisfy().unwrap(), 288);
}
