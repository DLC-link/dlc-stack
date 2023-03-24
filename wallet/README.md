# DLC-protocol-wallet

A test backend to:
* generating offer messages for numerical contracts
* receiving accept messages and returning sign messages from them

Run with:
```bash
cargo run
```

Run against dev test environment:

```bash
STORAGE_API_ENDPOINT="https://dev-oracle.dlc.link/storage-api" FUNDED_URL="https://stacks-observer-mocknet.herokuapp.com/funded" BTC_RPC_URL="electrs-btc2.dlc.link:18443/wallet/alice" RPC_USER="devnet2" RPC_PASS="devnet2" ORACLE_URL="https://dev-oracle.dlc.link/oracle" STORAGE_API_ENABLED=true RUST_LOG=warn,dlc_protocol_wallet=info cargo run
```

Run against a full local stack (dlc.link devs only):

```bash
STORAGE_API_ENDPOINT="http://localhost:8100" FUNDED_URL="http://localhost:8889/funded" BTC_RPC_URL="localhost:28443/wallet/alice" RPC_USER="devnet2" RPC_PASS="devnet2" ORACLE_URL="http://localhost:8080" RUST_BACKTRACE=full STORAGE_API_ENABLED=true RUST_LOG=warn,dlc_protocol_wallet=info cargo run
```

* Note, you can change the RUST_LOG to RUST_LOG=warn,dlc_protocol_wallet=debug for more debugging of this app's functioning.

Docker Compose example:

- go into docker folder and create a .env like this:


** Make updates here **
```
ORACLE_URL="https://dev-oracle.dlc.link"
BTC_RPC_URL="electrs-btc2.dlc.link:18443/wallet/alice"
RPC_USER="devnet2"
RPC_PASS="devnet2"
RUST_BACKTRACE=full
RUST_LOG=warn,dlc_protocol_wallet=info
STORAGE_API_ENABLED=true
STORAGE_API_ENDPOINT="https://dev-oracle.dlc.link/storage-api"
USE_SLED=false
FUNDED_URL="https://stacks-observer-mocknet.herokuapp.com/funded"
DOCKER_PUBLIC_REGISTRY_PREFIX=public.ecr.aws/dlc-link/
```

Then run:

```
docker-compose up -d
```

## API documentation:

See [wallet.yaml](docs/wallet.yaml) - the content can be copied to [swagger editor](https://editor.swagger.io/)
