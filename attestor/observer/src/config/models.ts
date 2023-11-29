export interface ChainConfig {
  network: Chain;
  version: string;
  api_key_required?: boolean;
  api_key?: string;
  endpoint: string;
  type: 'EVM' | 'STX';
  name: string;
}

export type EthChain = 'mainnet' | 'sepolia' | 'goerli' | 'localhost';
export type StacksChain = 'mainnet' | 'testnet' | 'mocknet' | 'local';
export type L2Chains = 'x1test';

export type Chain = EthChain | StacksChain | L2Chains;

export const validChains: Chain[] = [
  'mainnet',
  'sepolia',
  'goerli',
  'localhost',
  'testnet',
  'mocknet',
  'local',
  'x1test',
];
