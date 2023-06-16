// use serde::{Deserialize, Serialize};
// use std::env;
// use vaultrs::api::kv2::responses::SecretVersionMetadata;
// use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
// use vaultrs::error::ClientError;
// use vaultrs::kv2;

// /// A struct representing a connection to a Vault server.
// pub struct VaultConn {
//     /// The `VaultClient` instance used to communicate with the Vault server.
//     pub client: VaultClient,
// }

// impl VaultConn {
//     /// Creates a new `VaultConn` instance.
//     ///
//     /// # Arguments
//     ///
//     /// * `address` - The address of the Vault server.
//     /// * `token` - The token to authenticate with the Vault server.
//     ///
//     /// # Returns
//     ///
//     /// The newly created `VaultConn` instance.
//     pub fn new() -> Self {
//         let address = env::var("VAULT_ADDR").unwrap();
//         let token = env::var("VAULT_TOKEN").unwrap();
//         let client = VaultClient::new(
//             VaultClientSettingsBuilder::default()
//                 .address(address)
//                 .token(token)
//                 .build()
//                 .unwrap(),
//         )
//         .unwrap();
//         Self { client }
//     }
// }

// #[derive(Debug, Deserialize, Serialize, Clone)]
// #[serde(rename_all = "camelCase")]
// /// This struct represents an Oracle private key.
// ///
// /// It contains one field:
// ///
// /// - `private_key_value`: The private key string.
// pub struct OraclePrivateKey {
//     pub private_key_value: String,
// }

// /// Retrieves an Oracle private key from the Vault server.
// ///
// /// # Arguments
// ///
// /// * `secret_path` - The path to the secret in the Vault server.
// ///
// /// # Returns
// ///
// /// * `Result<OraclePrivateKey, ClientError>` - The `OraclePrivateKey` struct containing the private key, or an error if one occurred.
// pub async fn get_secret_key(
//     secret_path: &str,
//     secret_mount: &str,
// ) -> Result<OraclePrivateKey, ClientError> {
//     let client = VaultConn::new().client;
//     let secret: OraclePrivateKey = kv2::read(&client, secret_mount, secret_path).await?;
//     Ok(secret)
// }

// /// Sets an Oracle private key in the Vault server.
// ///
// /// # Arguments
// ///
// /// * `secret_path` - The path to the secret in the Vault server.
// /// * `value` - The `OraclePrivateKey` struct containing the private key and password.
// ///
// /// # Returns
// ///
// /// * `Result<OraclePrivateKey, ClientError>` - The `OraclePrivateKey` struct containing the private key, or an error if one occurred.
// pub async fn set_secret_key(
//     secret_path: &str,
//     secret_mount: &str,
//     value: OraclePrivateKey,
// ) -> Result<OraclePrivateKey, ClientError> {
//     let client = VaultConn::new().client;
//     let _: SecretVersionMetadata =
//         kv2::set(&client, secret_mount, secret_path, &value.clone()).await?;
//     Ok(value.clone())
// }
