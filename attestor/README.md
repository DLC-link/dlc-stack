# DLC.Link Attestor

A numeric oracle implementation for Bitcoin, with a built in blockchain-observer.

The Observer is configured to listen to specific events on the specified versions of the [DLCManager Contract (e.g. on ETH)](https://github.com/DLC-link/dlc-solidity/blob/master/contracts/DLCManager.sol).

When it hears a `Create`- or `CloseDLC` event, it will create an `Announcement` or `Attestation` through the imported wasm Attestor, respectively.

## Structure

The Attestor project is made up of two parts:

- A numeric DLC Attestor implementation, written in Rust, which compiles into WASM
- An Observer wrapper, which creates an interface between blockchain events and the Attestor, written as a Node.js app

## How to run

There are two ways to run this attestor. We have prebuilt docker images available on our AWS Container Registry, or one can pull this repository, and build & run from source.

> Important!
>
> If you wish to participate in the DLC.Link Attestation Layer, you must also register your attestor on our network. This requires a static public IP address, and the registration of this address on our smart contracts. Contact us for details.

### Option 1. Run using Docker

You will need docker installed on your machine. Fetch the preset docker-compose file:

```bash
$ wget https://github.com/DLC-link/dlc-stack/raw/master/attestor/docker-compose.yml
```

Note the environment variables set in this file. It is preset to a default configuration, listening to one particular chain. Attestors can listen to multiple chains simultaneously. See an example [here](./observer/.env.template).

> ! Important !
>
> When using Ethereum, you must provide an API key for Infura as an environment variable.
> (Option for listening to other providers/own nodes to be added later).

> You can provide your own ATTESTOR_XPRIV extended private key too, but if omitted, the attestor will generate one for you. Take good care of this key.

You can set the environment variables and start the service in one go using the following format:

```sh
$ INFURA_API_KEY=[your-infura-api-key] ATTESTOR_XPRIV=[your-extended-private-key] docker compose up
```

### Option 2. Build and run locally

## How to run the Attestor locally

1. create a `.env` based on the `.env.template` file
1. `npm install`
1. `npm run build`
1. `npm run dev` for auto-recompiling Typescript

## Key management (WIP)

### Attestor

Attestors need to have careful private key management. It is critical that the key is not lost or shared, as this jeopardizes this node of the layer.

Key rotation is also considered as a best practice, and is detailed in the setup process for an Attestor node. Although cycling keys can limit the potential damage of leaking a key, it is important that an Attestor node keep access to older keys until all Attestations for events built with a specific key are issued. This is because an Attestation must be signed with the same key used to create the Announcement.

DLC.Link recommends and pre-configures its nodes to use HashiCorp Key Vault software for key management and rotation.

### Observer

The Blockchain Observer needs _read_ access to the configured chains. Ethereum needs an API_KEY for Infura. Stacks is configured so it does not need a special key at the moment. See the .env.template file for a potential setup of multi-chain listening.

## API Description

### List all oracle events (announcements)

```sh
$ curl -X GET http://localhost:8801/events
```

This endpoint returns a JSON array of oracle event objects.

Output example:

```json
[
  {
    "event_id": "0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4",
    "uuid": "0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4",
    "rust_announcement_json": "{\"announcementSignature\":\"ed64cdbbd2a9e3e68067b12616c09317ebf788de666d412c092a9f7b11658841ee8f4663f6d0b38b5eb49ee636887bee2bf634e203a7c505a7f33b92fdb440be\",\"oraclePublicKey\":\"65b927bc31cc677373b963d69b0725520139cf55ff52a8ee890ba6565f868209\",\"oracleEvent\":{\"oracleNonces\":[\"c1da020c7d369d0cd5083ecbb950ce0f39d1d9aa35a027de6c62a439a4df5e30\",\"30d949dcb32925dc953c0a333a84c27c8a3db97447668d9b28afd8fc2b75be2e\",\"98e20355bccab3a5c3b2102e802d06779c683efe0f980738f2e0ac590d18e61e\",\"0d711b2394ed3ffd1bc21e77f1f758678ad92711f83fa1074611d1197635f59d\",\"f388125f9d1592ab52b8a8e04c5c3f5ff2b7b91163fccf857466c3b265be4ca7\",\"60a0cf1435043e82a17f82985c55735c72f6825b3e66f35b1ff64c3494d055df\",\"57d7012f21ec79e69cdcb4f751fd2cfd94dae423e806abaab2fd5637d54dafa7\",\"a7ef0956b87f3b91e0e90089a600fb9239f0c3169bf9a705c749b3f1f986925b\",\"00eaffb96db2f3db70015f83638923e929fa9f86ddd44db89bccb0f92d673459\",\"b0bf4c092b316e09ef07838fe75b6d75376dbef342c3fe09659c10da93144111\",\"16978e9dbb3ed677fe20adacad8d3d4be8fcea5c075d9f875874fa3a11c1d7d6\",\"322b83d07aa42064324632de07f266e2d8c95c0e1c416be7ec4619f6b8b85e86\",\"fe64bcef3eb1f12af5ade118c0cf98354e36da7c0e8e66ad7b8de38adcf9191f\",\"61fba7aee419775abc618eca87e3a06d86b734c2f92c9f7ea96aa07ec03b0203\"],\"eventMaturityEpoch\":1684399474,\"eventDescriptor\":{\"digitDecompositionEvent\":{\"base\":2,\"isSigned\":false,\"unit\":\"BTCUSD\",\"precision\":0,\"nbDigits\":14}},\"eventId\":\"0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4\"}}",
    "rust_announcement": "ed64cdbbd2a9e3e68067b12616c09317ebf788de666d412c092a9f7b11658841ee8f4663f6d0b38b5eb49ee636887bee2bf634e203a7c505a7f33b92fdb440be65b927bc31cc677373b963d69b0725520139cf55ff52a8ee890ba6565f868209fdd822fd021d000ec1da020c7d369d0cd5083ecbb950ce0f39d1d9aa35a027de6c62a439a4df5e3030d949dcb32925dc953c0a333a84c27c8a3db97447668d9b28afd8fc2b75be2e98e20355bccab3a5c3b2102e802d06779c683efe0f980738f2e0ac590d18e61e0d711b2394ed3ffd1bc21e77f1f758678ad92711f83fa1074611d1197635f59df388125f9d1592ab52b8a8e04c5c3f5ff2b7b91163fccf857466c3b265be4ca760a0cf1435043e82a17f82985c55735c72f6825b3e66f35b1ff64c3494d055df57d7012f21ec79e69cdcb4f751fd2cfd94dae423e806abaab2fd5637d54dafa7a7ef0956b87f3b91e0e90089a600fb9239f0c3169bf9a705c749b3f1f986925b00eaffb96db2f3db70015f83638923e929fa9f86ddd44db89bccb0f92d673459b0bf4c092b316e09ef07838fe75b6d75376dbef342c3fe09659c10da9314411116978e9dbb3ed677fe20adacad8d3d4be8fcea5c075d9f875874fa3a11c1d7d6322b83d07aa42064324632de07f266e2d8c95c0e1c416be7ec4619f6b8b85e86fe64bcef3eb1f12af5ade118c0cf98354e36da7c0e8e66ad7b8de38adcf9191f61fba7aee419775abc618eca87e3a06d86b734c2f92c9f7ea96aa07ec03b02036465e572fdd80a100002000642544355534400000000000e42307866626465323266616132633364626436383035383762356463663339656161663236376134656138303561616464633236313863313037613735623066376434",
    "rust_attestation_json": null,
    "rust_attestation": null,
    "maturation": "1684399474",
    "outcome": null
  }
]
```

### Get oracle event (announcement)

```sh
$ curl -X GET http://localhost:8801/event/0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4
```

This endpoint returns an [oracle event object](#list-all-oracle-events-announcements).

Output example:

```json
{
  "event_id": "0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4",
  "uuid": "0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4",
  "rust_announcement_json": "{\"announcementSignature\":\"ed64cdbbd2a9e3e68067b12616c09317ebf788de666d412c092a9f7b11658841ee8f4663f6d0b38b5eb49ee636887bee2bf634e203a7c505a7f33b92fdb440be\",\"oraclePublicKey\":\"65b927bc31cc677373b963d69b0725520139cf55ff52a8ee890ba6565f868209\",\"oracleEvent\":{\"oracleNonces\":[\"c1da020c7d369d0cd5083ecbb950ce0f39d1d9aa35a027de6c62a439a4df5e30\",\"30d949dcb32925dc953c0a333a84c27c8a3db97447668d9b28afd8fc2b75be2e\",\"98e20355bccab3a5c3b2102e802d06779c683efe0f980738f2e0ac590d18e61e\",\"0d711b2394ed3ffd1bc21e77f1f758678ad92711f83fa1074611d1197635f59d\",\"f388125f9d1592ab52b8a8e04c5c3f5ff2b7b91163fccf857466c3b265be4ca7\",\"60a0cf1435043e82a17f82985c55735c72f6825b3e66f35b1ff64c3494d055df\",\"57d7012f21ec79e69cdcb4f751fd2cfd94dae423e806abaab2fd5637d54dafa7\",\"a7ef0956b87f3b91e0e90089a600fb9239f0c3169bf9a705c749b3f1f986925b\",\"00eaffb96db2f3db70015f83638923e929fa9f86ddd44db89bccb0f92d673459\",\"b0bf4c092b316e09ef07838fe75b6d75376dbef342c3fe09659c10da93144111\",\"16978e9dbb3ed677fe20adacad8d3d4be8fcea5c075d9f875874fa3a11c1d7d6\",\"322b83d07aa42064324632de07f266e2d8c95c0e1c416be7ec4619f6b8b85e86\",\"fe64bcef3eb1f12af5ade118c0cf98354e36da7c0e8e66ad7b8de38adcf9191f\",\"61fba7aee419775abc618eca87e3a06d86b734c2f92c9f7ea96aa07ec03b0203\"],\"eventMaturityEpoch\":1684399474,\"eventDescriptor\":{\"digitDecompositionEvent\":{\"base\":2,\"isSigned\":false,\"unit\":\"BTCUSD\",\"precision\":0,\"nbDigits\":14}},\"eventId\":\"0xfbde22faa2c3dbd680587b5dcf39eaaf267a4ea805aaddc2618c107a75b0f7d4\"}}",
  "rust_announcement": "ed64cdbbd2a9e3e68067b12616c09317ebf788de666d412c092a9f7b11658841ee8f4663f6d0b38b5eb49ee636887bee2bf634e203a7c505a7f33b92fdb440be65b927bc31cc677373b963d69b0725520139cf55ff52a8ee890ba6565f868209fdd822fd021d000ec1da020c7d369d0cd5083ecbb950ce0f39d1d9aa35a027de6c62a439a4df5e3030d949dcb32925dc953c0a333a84c27c8a3db97447668d9b28afd8fc2b75be2e98e20355bccab3a5c3b2102e802d06779c683efe0f980738f2e0ac590d18e61e0d711b2394ed3ffd1bc21e77f1f758678ad92711f83fa1074611d1197635f59df388125f9d1592ab52b8a8e04c5c3f5ff2b7b91163fccf857466c3b265be4ca760a0cf1435043e82a17f82985c55735c72f6825b3e66f35b1ff64c3494d055df57d7012f21ec79e69cdcb4f751fd2cfd94dae423e806abaab2fd5637d54dafa7a7ef0956b87f3b91e0e90089a600fb9239f0c3169bf9a705c749b3f1f986925b00eaffb96db2f3db70015f83638923e929fa9f86ddd44db89bccb0f92d673459b0bf4c092b316e09ef07838fe75b6d75376dbef342c3fe09659c10da9314411116978e9dbb3ed677fe20adacad8d3d4be8fcea5c075d9f875874fa3a11c1d7d6322b83d07aa42064324632de07f266e2d8c95c0e1c416be7ec4619f6b8b85e86fe64bcef3eb1f12af5ade118c0cf98354e36da7c0e8e66ad7b8de38adcf9191f61fba7aee419775abc618eca87e3a06d86b734c2f92c9f7ea96aa07ec03b02036465e572fdd80a100002000642544355534400000000000e42307866626465323266616132633364626436383035383762356463663339656161663236376134656138303561616464633236313863313037613735623066376434",
  "rust_attestation_json": null,
  "rust_attestation": null,
  "maturation": "1684399474",
  "outcome": null
}
```

### Get public key

```sh
$ curl -X GET http://localhost:8801/publickey
```

This endpoint returns the `public_key` of the attestor.
