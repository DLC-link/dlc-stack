export interface ChainConfig {
  network: Chain;
  version: string;
  api_key_required?: boolean;
  api_key?: string;
  endpoint: string;
  type: 'EVM' | 'STX';
  name: string;
}

export type EthChain = 'ETH_MAINNET' | 'ETH_SEPOLIA' | 'ETH_GOERLI' | 'ETH_LOCAL';
export type StacksChain = 'STACKS_MAINNET' | 'STACKS_TESTNET' | 'STACKS_MOCKNET' | 'STACKS_LOCAL';
export type L2Chains = 'OKX_TESTNET';

export type Chain = EthChain | StacksChain | L2Chains;

export const validChains: Chain[] = [
  'ETH_MAINNET',
  'ETH_SEPOLIA',
  'ETH_GOERLI',
  'ETH_LOCAL',
  'STACKS_MAINNET',
  'STACKS_TESTNET',
  'STACKS_MOCKNET',
  'STACKS_LOCAL',
  'OKX_TESTNET',
];
