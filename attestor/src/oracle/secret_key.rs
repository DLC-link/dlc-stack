// use crate::oracle::vault::{get_secret_key, set_secret_key, OraclePrivateKey};
// use gethostname::gethostname;
// use log::info;
// use log::warn;
// use secp256k1_zkp::{rand, All, KeyPair, Secp256k1, SecretKey};
// use std::env;
// use std::fs::File;
// use std::io::prelude::*;
// use std::path::Path;
// use std::str::FromStr;
// use vaultrs::error::ClientError;

// /// This function returns a `KeyPair` for the given `Secp256k1` context.
// ///
// /// It first checks the `VAULT_ENABLED` environment variable to determine
// /// whether to get the secret key from Vault or from the local configuration.
// /// If `VAULT_ENABLED` is set to `true`, it calls `get_or_generate_secret_from_vault`
// /// to get the secret key from Vault. Otherwise, it calls `get_or_generate_secret_from_config`
// /// to get the secret key from the local configuration.
// ///
// /// Once the secret key has been obtained, this function returns a `KeyPair`
// /// created from the secret key and the given `Secp256k1` context.
// ///
// /// # Arguments
// ///
// /// * `secp` - A reference to a `Secp256k1` context.
// /// * `secret_key_file` - An optional `PathBuf` that specifies the path to the secret key file.
// ///
// /// # Returns
// ///
// /// A `KeyPair` created from the secret key and the given `Secp256k1` context.
// pub async fn get_or_generate_keypair(
//     secp: &Secp256k1<All>,
//     secret_key_file: Option<std::path::PathBuf>,
// ) -> KeyPair {
//     let use_vault: bool = env::var("VAULT_ENABLED")
//         .unwrap_or("false".to_string())
//         .parse()
//         .unwrap();
//     let secret_key: SecretKey;
//     if use_vault {
//         secret_key = get_or_generate_secret_from_vault(&secp).await.unwrap();
//     } else {
//         secret_key = get_or_generate_secret_from_config(&secp, secret_key_file).unwrap();
//     }
//     return KeyPair::from_secret_key(&secp, &secret_key);
// }

// async fn get_or_generate_secret_from_vault(secp: &Secp256k1<All>) -> anyhow::Result<SecretKey> {
//     let vault_data_path: String = env::var("VAULT_DATA_PATH").unwrap_or("".to_string());
//     let oracle_key = match vault_data_path.is_empty() {
//         true => format!("oracle/{}", gethostname().to_string_lossy()),
//         false => format!("oracle/{}", vault_data_path),
//     };
//     let secret_mount = "secret";
//     let vault_key: OraclePrivateKey = match get_secret_key(&oracle_key, secret_mount).await {
//         Ok(res) => res,
//         Err(err) => match err {
//             ClientError::APIError { code, errors } => {
//                 if code == 404 {
//                     let new_key = secp.generate_keypair(&mut rand::thread_rng()).0;
//                     let secret_str = new_key.display_secret().to_string();
//                     let res = set_secret_key(
//                         &oracle_key,
//                         secret_mount,
//                         OraclePrivateKey {
//                             private_key_value: secret_str,
//                         },
//                     )
//                     .await?;
//                     warn!("Resource not found: {}", errors.join(","));
//                     res
//                 } else {
//                     panic!("Unexpected client error. Exiting ...");
//                 }
//             }
//             _ => {
//                 panic!("Unexpected server error. Exiting ...");
//             }
//         },
//     };
//     Ok(SecretKey::from_str(&vault_key.private_key_value)?)
// }

// fn get_or_generate_secret_from_config(
//     secp: &Secp256k1<All>,
//     secret_key_file: Option<std::path::PathBuf>,
// ) -> anyhow::Result<SecretKey> {
//     let mut secret_key = String::new();
//     let secret_key = match secret_key_file {
//         None => {
//             let path = Path::new("config/secret.key");
//             if path.exists() {
//                 info!(
//                     "reading secret key from {} (default)",
//                     path.file_name().unwrap().to_string_lossy()
//                 );
//                 File::open(path)?.read_to_string(&mut secret_key)?;
//                 secret_key.retain(|c| !c.is_whitespace());
//                 SecretKey::from_str(&secret_key)?
//             } else {
//                 info!("no secret key file was found, generating secret key");
//                 let new_key = secp.generate_keypair(&mut rand::thread_rng()).0;
//                 let mut file = File::create("config/secret.key")?;
//                 file.write_all(new_key.display_secret().to_string().as_bytes())?;
//                 new_key
//             }
//         }
//         Some(path) => {
//             info!(
//                 "reading secret key from {}",
//                 path.as_os_str().to_string_lossy()
//             );
//             File::open(path)?.read_to_string(&mut secret_key)?;
//             secret_key.retain(|c| !c.is_whitespace());
//             SecretKey::from_str(&secret_key)?
//         }
//     };
//     Ok(secret_key)
// }
