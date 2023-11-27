import dotenv from 'dotenv';
dotenv.config();

const env = process.env.ENV || 'devnet';

const devnet = {
  testWalletPrivateKey: 'b5984262748203b2043923dd34202d1a6e05601af0c00e232d3b1988ce9608f5',
  testWalletAddress: 'bcrt1qpnuck30uakpc0ffcmd3nwdd59y547qlzsmf34l',
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  // TODO: which wallet?
  protocolWalletURL: 'https://devnet.dlc.link/eth-wallet',
  attestorList: [
    'https://devnet.dlc.link/attestor-1',
    'https://devnet.dlc.link/attestor-2',
    'https://devnet.dlc.link/attestor-3',
  ],
  storageApiUrl: 'https://devnet.dlc.link/storage-api',
};

const testnet = {
  //  TODO: privatekey on testnet?
  testWalletPrivateKey: 'bea4ecfec5cfa1e965ee1b3465ca4deff4f04b36a1fb5286a07660d5158789fb',
  testWalletAddress: 'tb1q3tj2fr9scwmcw3rq5m6jslva65f2rqjxt2t0zh',
  bitcoinNetwork: 'testnet',
  bitcoinNetworkURL: 'https://testnet.dlc.link/electrs',
  // TODO: which wallet?
  protocolWalletURL: 'https://testnet.dlc.link/stacks-wallet',
  attestorList: [
    'https://testnet.dlc.link/attestor-1',
    'https://testnet.dlc.link/attestor-2',
    'https://testnet.dlc.link/attestor-3',
  ],
  storageApiUrl: 'https://testnet.dlc.link/storage-api',
};

// Local services, but regtest bitcoin
const local = {
  testWalletPrivateKey: 'b5984262748203b2043923dd34202d1a6e05601af0c00e232d3b1988ce9608f5',
  testWalletAddress: 'bcrt1qpnuck30uakpc0ffcmd3nwdd59y547qlzsmf34l',
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  protocolWalletURL: 'http://127.0.0.1:3003',
  attestorList: ['http://localhost:8801', 'http://localhost:8802', 'http://localhost:8803'],
  storageApiUrl: 'http://127.0.0.1:8100',
};

// Local services with just script, but regtest bitcoin
const local_just = {
  testWalletPrivateKey: 'b5984262748203b2043923dd34202d1a6e05601af0c00e232d3b1988ce9608f5',
  testWalletAddress: 'bcrt1qpnuck30uakpc0ffcmd3nwdd59y547qlzsmf34l',
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  protocolWalletURL: 'http://127.0.0.1:3003',
  attestorList: ['http://127.0.0.1:8801', 'http://127.0.0.1:8802', 'http://127.0.0.1:8803'],
  storageApiUrl: 'http://127.0.0.1:8100',
};

const docker = {
  testWalletPrivateKey: 'b5984262748203b2043923dd34202d1a6e05601af0c00e232d3b1988ce9608f5',
  testWalletAddress: 'bcrt1qpnuck30uakpc0ffcmd3nwdd59y547qlzsmf34l',
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  protocolWalletURL: 'http://172.20.128.3:3003',
  attestorList: ['http://172.20.128.5:8801', 'http://172.20.128.6:8802', 'http://172.20.128.7:8803'],
  storageApiUrl: 'http://172.20.128.1:8100',
};

const custom = {
  testWalletPrivateKey: devnet.testWalletPrivateKey,
  testWalletAddress: devnet.testWalletAddress,
  bitcoinNetwork: devnet.bitcoinNetwork,
  bitcoinNetworkURL: devnet.bitcoinNetworkURL,
  protocolWalletURL: local.protocolWalletURL,
  attestorList: devnet.attestorList,
  storageApiUrl: devnet.storageApiUrl,
};

const config = {
  devnet,
  testnet,
  local,
  local_just,
  docker,
  custom,
};

export default config[env];
