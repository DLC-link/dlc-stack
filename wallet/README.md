# DLC.Link Router Wallet
A DLC enabled bitcoin wallet which supports the creation and signing of DLCs via an HTTP server.

This application is written in Rust and is intended to compile to a binary and run directly as a service. It also has a docker file and is ready to be run as a container.

## Structure
The DLC.Link Router Wallet acts as the counter-party to a DLC and provides the following features:

- Create, and later sign, a DLC offer (following the flow of the DLC Offering party)
- Send a message to a smart contract that the DLC has been funded (TODO: Link to dlc-funded message details)
- Periodically check if any of the DLCs currently open are ready to be closed (the Attestors have attested)
- Send a message to a smart contract that the DLC has been closed. (TODO: Link to post-close message details)

The send messages in features 2 & 4 will send a message to the DLC.Link DLC-Manager smart contract, which can in-turn provide a callback to a the contract which originally set up the contract. Learn more by reviewing the docs for your blockchain for [Ethereum](https://github.com/DLC-link/dlc-solidity) or [Stacks](https://github.com/DLC-link/dlc-clarity)

This application currently only supports the creation of a DLC offer, and then later signing that offer. It does not support "accepting" an offer from another DLC wallet. Learn more about the DLC flow secification here: https://github.com/discreetlogcontracts/dlcspecs/blob/master/Protocol.md

This application is intended to function as a service of a decentralized application (dapp). Because of this, this application never posts collateral into the DLC. It also does not pay any gas fee related to the DLC (funding tx nor any of the CET outcomes).

## Routing funds
This application is intended to be a counterparty in a DLC for a dapp, and then "route" any funds it gets to a payout bitcoin address in an automated way upon DLC closing. This can be done in two ways:

### Option 1. External funding address for DLCs
The routing happens as a built-in mechanism of the DLC, as this application will put a given payout address in as the payout address of the DLC (in the CETs). This way, the funds of the DLC never actually go to this application, but rather to the payout address directly. In this case, the wallet of this application never receives or needs any Bitcoin, and just acts as an automated counterparty of a DLC.

### Option 2. Manual funds routing
If this application's bitcoin-wallet address is used as the funding output address of the DLC, then the funds that go to this wallet will need to be manually moved to some other location, such as another wallet, or a bridge, etc.

## How to run/build
The DLC.Link Router Wallet is indended to be run as a stand alone service.We have prebuilt docker images available on our AWS Container Registry, or one can pull this repository, and build & run from source.

### Wallet Blockchain Interface
The Router requires the companion service knows as the Wallet Blockchain Interface to be running as well, as it will look for this on startup and during various functions.

Some functions of the DLC.Link Router Wallet require accessing a smart-contract on a corresponding blockchain which supports smart contracts. Currently supported are Ethereum and Stacks. This is done via a proxying JS service which runs separately, as a companion to the Router Wallet application.

Learn more about this tool here [WBI-Readme](https://github.com/DLC-link/dlc-stack/tree/master/wallet/wallet-blockchain-interface)

### Generate a Key
Use [Just](https://github.com/casey/just) to generate a key and a cooresponding fingerprint for your wallet. Back this up securely. Pass this into the env vars as described below. This takes an env variable for BITCOIN_NETWORK.

You can run with the following command
```sh
$ BITCOIN_NETWORK=regtest cargo run --bin generate-key
```

### Setup ENV vars

The following environment variables must be passed into this application, whether running as docker or from source.

* BITCOIN_NETWORK: "regtest" # regtest / sigtest / testnet / bitcoin
* BLOCKCHAIN_INTERFACE_URL: "localhost:3003" # URL to a companion service called the Wallet Blockchain Interface. Learn more here: https://github.com/DLC-link/dlc-stack/tree/dev/wallet-blockchain-interface
* CELECTRUM_API_URL: "https://blockstream.info/testnet/api/" # URL to an Esplora bitcoin API
* FINGERPRINT: "3a64ca13" # The key fingerprint generated when running the Generate Key binary. See [here](#generate-a-key)
* RUST_LOG: "info,dlc_protocol_wallet=debug,dlc_clients=debug,bdk=debug,dlc_manager=debug,electrs_blockchain_provider=debug,dlc_bdk_wallet=debug,esplora_async_blockchain_provider=trace" # Different logging levels for each package is supported.
* RUST_BACKTRACE: "full" # Show a full backtrace in case of panic.
* SLED_WALLET_PATH": "wallet_db" # Directory name for storing a local cache of the bitcoin wallet's data.
* STORAGE_API_ENABLED: "true" # Use the cloud database for the DLCs managed by DLC.Link. Recommended.
* STORAGE_API_ENDPOINT: "https://devnet.dlc.link/storage-api" # URL for the cloud database.
* XPRIVATE_KEY: "tprv8Z..." # The private key generated when running the Generate Key binary. See [here](#generate-a-key)

### Option 1. Run using Docker

>! Note !
>
> If you run into an authentication error when pulling down the docker image like this:
> `Error response from daemon: pull access denied for public.ecr.aws/dlc-link/dlc-protocol-wallet, repository does not exist or may require 'docker login': denied: Your authorization token has expired. Reauthenticate and try again`
>
> Run the authentication command like this:
> `aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws`
>
> as per this article: https://docs.aws.amazon.com/AmazonECR/latest/public/public-registries.html#public-registry-auth

You will need docker installed on your machine. Fetch the preset docker-compose file:

```bash
$ wget https://github.com/DLC-link/dlc-stack/raw/master/wallet/docker-compose.yml
```

You can set the environment variables and start the service in one go using the following format:
```sh
$ BITCOIN_NETWORK=[network] BLOCKCHAIN_INTERFACE_URL=[...] <etc>  docker compose up
```

### Option 2. Build and run locally

#### Dev Build
```sh
$ BITCOIN_NETWORK=[network] BLOCKCHAIN_INTERFACE_URL=[...] <etc>  cargo run
```

#### Production Build
You can build a binary for production like this:
`cargo build --release --bins --target-dir .` Then run the following:
```sh
$ BITCOIN_NETWORK=[network] BLOCKCHAIN_INTERFACE_URL=[...] <etc>  ./router-wallet
```
