import dotenv from 'dotenv';
dotenv.config();

const mainnet = {
  network: 'mainnet',
  admin_private_key: process.env.ETH_ADMIN_PRIVATE_KEY as string,
  infura_key: process.env.INFURA_KEY as string,
  providerURL: `wss://mainnet.infura.io/ws/v3/${process.env.INFURA_KEY as string}`,
  providerType: 'ws',
};

const sepolia = {
  network: 'sepolia',
  admin_private_key: process.env.ETH_ADMIN_PRIVATE_KEY as string,
  infura_key: process.env.INFURA_KEY as string,
  providerURL: `wss://sepolia.infura.io/ws/v3/${process.env.INFURA_KEY as string}`,
  providerType: 'ws',
};

const goerli = {
  network: 'goerli',
  admin_private_key: process.env.ETH_ADMIN_PRIVATE_KEY as string,
  infura_key: process.env.INFURA_KEY as string,
  providerURL: `wss://goerli.infura.io/ws/v3/${process.env.INFURA_KEY as string}`,
  providerType: 'ws',
};

const localhost = {
  network: 'localhost',
  admin_private_key: process.env.ETH_ADMIN_PRIVATE_KEY as string,
  infura_key: process.env.INFURA_KEY as string,
  providerURL: `http://127.0.0.1:8545`,
  providerType: 'jsonrpc',
};

export const environments = {
  mainnet,
  sepolia,
  goerli,
  localhost,
};
