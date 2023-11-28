import { ChainConfig } from './models.js';

const MOCKNET_ADDRESS = process.env.MOCKNET_ADDRESS || 'stx-btc1.dlc.link';
const currentDefaultVersion = '1';

export const defaultConfigs: Array<ChainConfig> = [
  {
    network: 'ETH_MAINNET',
    version: currentDefaultVersion,
    api_key_required: true,
    endpoint: 'wss://mainnet.infura.io/ws/v3/',
    type: 'EVM',
    name: 'mainnet',
  },
  {
    network: 'ETH_SEPOLIA',
    version: currentDefaultVersion,
    api_key_required: true,
    endpoint: 'wss://sepolia.infura.io/ws/v3/',
    type: 'EVM',
    name: 'sepolia',
  },
  {
    network: 'ETH_GOERLI',
    version: currentDefaultVersion,
    api_key_required: true,
    endpoint: 'wss://goerli.infura.io/ws/v3/',
    type: 'EVM',
    name: 'goerli',
  },
  {
    network: 'ETH_LOCAL',
    version: currentDefaultVersion,
    api_key_required: false,
    endpoint: 'http://localhost:8545',
    type: 'EVM',
    name: 'localhost',
  },
  {
    network: 'OKX_TESTNET',
    version: currentDefaultVersion,
    api_key_required: false,
    endpoint: 'wss://x1testws.okx.com',
    type: 'EVM',
    name: 'X1test',
  },
  {
    network: 'STACKS_MAINNET',
    version: currentDefaultVersion,
    api_key_required: false,
    endpoint: 'wss://api.hiro.so/',
    type: 'STX',
    name: 'stx-mainnet',
  },
  {
    network: 'STACKS_TESTNET',
    version: currentDefaultVersion,
    api_key_required: false,
    endpoint: 'wss://api.testnet.hiro.so/',
    type: 'STX',
    name: 'stx-testnet',
  },
  {
    network: 'STACKS_MOCKNET',
    version: currentDefaultVersion,
    api_key_required: false,
    endpoint: `ws://${MOCKNET_ADDRESS}:3999/`,
    type: 'STX',
    name: 'stx-mocknet',
  },
  {
    network: 'STACKS_LOCAL',
    version: currentDefaultVersion,
    api_key_required: false,
    endpoint: 'ws://localhost:3999',
    type: 'STX',
    name: 'stx-local',
  },
];
