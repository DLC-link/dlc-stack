export interface ChainConfig {
  network: Chain;
  version: string;
  api_key?: string;
  deployer?: string;
  endpoint: string;
}

export const evmPrefix = 'evm-';
export type EthChain = 'mainnet' | 'sepolia' | 'goerli' | 'localhost';
export type L2Chains = 'x1test';

export const stxPrefix = 'stx-';
export type StacksChain = 'mainnet' | 'testnet' | 'mocknet' | 'local';

export type Chain = EthChain | StacksChain | L2Chains;

export type PrefixedChain = `${'evm-' | 'stx-'}${Chain}`;

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
