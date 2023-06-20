import dotenv from 'dotenv';
dotenv.config();

import { StacksMainnet, StacksMocknet, StacksTestnet } from '@stacks/network';
import { NetworkConfig } from '../models/interfaces/network-config.interface';

const env = process.env.NETWORK as 'mocknet' | 'testnet' | 'mainnet' | 'mocknet_cloud';

const mocknet: NetworkConfig = {
  network: new StacksMocknet(),
  api_base: 'http://localhost:3999',
  api_base_extended: 'http://localhost:3999/extended/v1',
  ioclient_uri: 'ws://localhost:3999/',
  admin_address: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM',
  admin_private_key: '753b7cc01a1a2e86221266a154af739463fce51219d97e4f856cd7200c3bd2a601',
};

const mocknet_cloud: NetworkConfig = {
  network: new StacksMocknet({
    url: `https://${process.env.MOCKNET_ADDRESS}`,
  }),
  api_base: `https://${process.env.MOCKNET_ADDRESS}`,
  api_base_extended: `https://${process.env.MOCKNET_ADDRESS}/extended/v1`,
  ioclient_uri: `wss://${process.env.MOCKNET_ADDRESS}/`,
  admin_address: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM',
  admin_private_key: '753b7cc01a1a2e86221266a154af739463fce51219d97e4f856cd7200c3bd2a601',
};

const testnet: NetworkConfig = {
  network: new StacksTestnet(),
  api_base: 'https://api.testnet.hiro.so',
  api_base_extended: 'https://api.testnet.hiro.so/extended/v1',
  ioclient_uri: 'wss://api.testnet.hiro.so/',
  admin_address: process.env.ADMIN_ADDRESS as string,
  admin_private_key: process.env.ADMIN_PRIVATE_KEY as string,
};

const mainnet: NetworkConfig = {
  network: new StacksMainnet(),
  api_base: 'https://api.hiro.so',
  api_base_extended: 'https://api.hiro.so/extended/v1',
  ioclient_uri: 'wss://api.hiro.so/',
  admin_address: process.env.ADMIN_ADDRESS as string,
  admin_private_key: process.env.ADMIN_PRIVATE_KEY as string,
};

const environments = {
  mocknet,
  mocknet_cloud,
  testnet,
  mainnet,
};

export const config: NetworkConfig = environments[env];
