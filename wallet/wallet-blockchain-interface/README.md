# wallet-blockchain-interface

This module is responsible for communicating between the `dlc-manager` contracts on the smart-chain side, and the Attestor Layer.

## How it works

It provides an API for creating and managing the DLCs of a given `protocol-wallet`, described in the `wallet` module of this repository [here](../wallet/). It is this running `wallet` module that will be calling the endpoints.

The app is exposing four endpoints (or five in dev mode):
- GET `/health` - a simple health check endpoint
- POST `/set-status-funded` - sets the status of a given DLC to `funded`
- GET `/get-all-attestors` - returns all the attestors that are currently registered on the smart contract
- GET `/get-all-attestors-test` - returns a fixed list of attestors (for testing purposes only)
- POST `/post-close-dlc` - used as a callback after successful DLC attestation

See the endpoint definitions [here](./src/http/routes.ts).

Depending on the environment setup, it communicates with different Blockchains (one at a time). Take a look at the [`blockchain-writer.service.ts`](./src/services/blockchain-writer.service.ts) file to see how it works.


## How to run

### 1. As a standalone NodeJs application

> Create a `.env` file in this folder, and set the variables there, based on the `.env.template` file.

In this folder, run:

```bash
npm install
npm run build
npm run start
# or for a dev build
npm run dev
```

### 2. Docker Compose through `make`

In the root of the repository, run:

```bash
make docker-start
```

This will build and start all the services at once, including an instance of this `blockchain-interface`.
