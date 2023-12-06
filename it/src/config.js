import dotenv from 'dotenv';
dotenv.config();

const env = process.env.ENV || 'devnet';

// These are some funded regtest wallets that can be used for testing. They each have 0.1 BTC starting balance.
const someFundedRegtestWallets = [
  {
    privateKey: '0f4486431f2c33068a1df80c1b46efb1e0dfdd9b338b1554b86ee5c80134501d',
    address: 'bcrt1qcnr4g23wp7m08xhquddcwnehndmhw5kvas7h27',
  },
  {
    privateKey: '9bc0ec04a03f4acb98ccbbfcbf8c702df2a77a1ab87494c98c7c7980686554dc',
    address: 'bcrt1qtdp36gg86xacn984825arsqhh4ge7yza6rm0gt',
  },
  {
    privateKey: '43194270c95daa17ed5ead7e72136f4267411c57bf87d4889546a2e864507554',
    address: 'bcrt1qve066efyw9wr23k0ge03m30w0t3dtt08dju58k',
  },
  {
    privateKey: 'f6eae6340a63bc08288f6eadb79a55d9503a9ed3c91c124e294b17747e3ec5c3',
    address: 'bcrt1qkm6dqj654evqpfwlqrr0djdgl7q3c7gv4lug72',
  },
  {
    privateKey: 'd39e788fee4a3cbdc515ab700dc2cb679153d01a9636d129650feef9965d67e2',
    address: 'bcrt1qjvpjq4pa9k9klknxf0qt5y76u9uc6q7ayxaupw',
  },
  {
    privateKey: '2e78040c9d61db7d776a6c75c48cb4649b3877a088c9009abb9b9960f80e9806',
    address: 'bcrt1qw8f2sm0qqdsrecx26vzuf4ssq8fq7x5svqtl8t',
  },
  {
    privateKey: 'fa6dbda19773aedeeb9fe27d5c8b09a1b0603d108dee53077e2c0f043317a486',
    address: 'bcrt1qqygkj3usflkel5m7z8fwlnx5r3acel5f7tq47p',
  },
  {
    privateKey: '13e91c250de5d08b0614943b6f6f612f25a52ac746a0cb024373207c13b3576e',
    address: 'bcrt1qwauw7z4n9csjrq4ytsj8psmreh2dxtsz49pxe2',
  },
  {
    privateKey: '0e45bacc4a0905e0eae05ce66fb3be46026bac4aeb6b78e9d8013bc2c78556f7',
    address: 'bcrt1qpvpqa8pz3wr2624fcqzagsrhzk023awhcvjadq',
  },
  {
    privateKey: '8bce11ce7340ff6e779076707d83dfd9c623fc9376f2e9030113c5b61325a057',
    address: 'bcrt1qevw66czdu2zy0wm7wuep6ku6qcrh6zzls9y69k',
  },
];

const randomIndex = Math.floor(Math.random() * someFundedRegtestWallets.length);
console.log('[IT] randomIndex', randomIndex);

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
  testWalletPrivateKey: process.env.TEST_WALLET_PKEY,
  testWalletAddress: process.env.TEST_WALLET_ADDRESS,
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  protocolWalletURL: 'http://127.0.0.1:3003',
  attestorList: ['http://localhost:8801', 'http://localhost:8802', 'http://localhost:8803'],
  storageApiUrl: 'http://127.0.0.1:8100',
};

// Local services with just script, but regtest bitcoin
const local_just = {
  testWalletPrivateKey: process.env.TEST_WALLET_PKEY,
  testWalletAddress: process.env.TEST_WALLET_ADDRESS,
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  protocolWalletURL: 'http://127.0.0.1:3003',
  attestorList: ['http://127.0.0.1:8801', 'http://127.0.0.1:8802', 'http://127.0.0.1:8803'],
  storageApiUrl: 'http://127.0.0.1:8100',
};

const docker = {
  testWalletPrivateKey: someFundedRegtestWallets[randomIndex].privateKey,
  testWalletAddress: someFundedRegtestWallets[randomIndex].address,
  bitcoinNetwork: 'regtest',
  bitcoinNetworkURL: 'https://devnet.dlc.link/electrs',
  protocolWalletURL: 'http://172.20.128.2:3003',
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
