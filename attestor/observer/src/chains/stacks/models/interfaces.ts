import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';

///////////////////////// Nfts
export interface Value {
  hex: string;
  repr: string;
}

export interface Result {
  asset_identifier: string;
  value: Value;
  block_height: number;
  tx_id: string;
}
export interface NFTHoldingsData {
  limit: number;
  offset: number;
  total: number;
  results: Result[];
}

///////////////////////// Contracts

export interface AddressSubscription {
  address: string;
  subscription: { unsubscribe: () => void };
  handleTx: (tx: ContractCallTransaction) => void;
}

export interface DeploymentInfo {
  deployer: string;
  api_base_extended: string;
}

export interface ContractConfig {
  // contractFullName: string;
  init: () => Promise<void>;
  registeredContractSubscriptions: Array<AddressSubscription>;
  checkAddresses: (address: string) => boolean;
  handleTx: (tx: ContractCallTransaction) => Promise<void>;
}

export interface NetworkConfig {
  socket: StacksApiSocketClient;
  deploymentInfo: DeploymentInfo;
}

///////////////////////// Transactions

export type FunctionName =
  | 'create-dlc'
  | 'close-dlc'
  | 'register-contract'
  | 'unregister-contract'
  | 'set-status-funded'
  | 'post-create-dlc'
  | 'post-close-dlc'
  | 'get-btc-price'
  | 'validate-price-data';

export type ArgumentName =
  | 'uuid'
  | 'outcome'
  | 'callback-contract'
  | 'creator'
  | 'protocol-wallet'
  | 'contract-address'
  | 'attestors'
  | 'actual-closing-time'
  | 'event-source';

export type UnwrappedPrintEvent = {
  [arg in ArgumentName]?: {
    value: any;
  };
};
