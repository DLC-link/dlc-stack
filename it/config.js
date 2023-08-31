import dotenv from "dotenv";
dotenv.config();

const env = process.env.ENV || "devnet";

const devnet = {
  testWalletPrivateKey:
    "bea4ecfec5cfa1e965ee1b3465ca4deff4f04b36a1fb5286a07660d5158789fb",
  testWalletAddress: "bcrt1q3tj2fr9scwmcw3rq5m6jslva65f2rqjxfrjz47",
  bitcoinNetwork: "regtest",
  bitcoinNetworkURL: "https://devnet.dlc.link/electrs",
  // TODO: which wallet?
  protocolWalletURL: "https://devnet.dlc.link/stacks-wallet",
  attestorList: [
    "https://devnet.dlc.link/attestor-1",
    "https://devnet.dlc.link/attestor-2",
    "https://devnet.dlc.link/attestor-3",
  ],
};

const testnet = {
  //  TODO: privatekey on testnet?
  testWalletPrivateKey:
    "bea4ecfec5cfa1e965ee1b3465ca4deff4f04b36a1fb5286a07660d5158789fb",
  testWalletAddress: "tb1q3tj2fr9scwmcw3rq5m6jslva65f2rqjxt2t0zh",
  bitcoinNetwork: "testnet",
  bitcoinNetworkURL: "https://testnet.dlc.link/electrs",
  // TODO: which wallet?
  protocolWalletURL: "https://testnet.dlc.link/stacks-wallet",
  attestorList: [
    "https://testnet.dlc.link/attestor-1",
    "https://testnet.dlc.link/attestor-2",
    "https://testnet.dlc.link/attestor-3",
  ],
};

// Local services, but regtest bitcoin
const local = {
  testWalletPrivateKey:
    "bea4ecfec5cfa1e965ee1b3465ca4deff4f04b36a1fb5286a07660d5158789fb",
  testWalletAddress: "bcrt1q3tj2fr9scwmcw3rq5m6jslva65f2rqjxfrjz47",
  bitcoinNetwork: "regtest",
  bitcoinNetworkURL: "https://devnet.dlc.link/electrs",
  protocolWalletURL: "http://localhost:8085",
  attestorList: [
    "http://localhost:8801",
    "http://localhost:8802",
    "http://localhost:8803",
  ],
};

const docker = {
  testWalletPrivateKey:
    "bea4ecfec5cfa1e965ee1b3465ca4deff4f04b36a1fb5286a07660d5158789fb",
  testWalletAddress: "bcrt1q3tj2fr9scwmcw3rq5m6jslva65f2rqjxfrjz47",
  bitcoinNetwork: "regtest",
  bitcoinNetworkURL: "https://devnet.dlc.link/electrs",
  protocolWalletURL: "http://host.docker.internal:8085",
  attestorList: [
    "http://host.docker.internal:8801",
    "http://host.docker.internal:8802",
    "http://host.docker.internal:8803",
  ],
};

const custom = {
  testWalletPrivateKey: process.env.TEST_WALLET_PRIVATE_KEY,
  testWalletAddress: process.env.TEST_WALLET_ADDRESS,
  bitcoinNetwork: process.env.BITCOIN_NETWORK,
  bitcoinNetworkURL: process.env.BITCOIN_NETWORK_URL,
  protocolWalletURL: process.env.PROTOCOL_WALLET_URL,
  attestorList: process.env.ATTESTOR_LIST.split(","),
};

const config = {
  devnet,
  testnet,
  local,
  docker,
  custom,
};

export default config[env];
