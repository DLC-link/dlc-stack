import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { TxBroadcastResult } from '@stacks/transactions';
import { PrefixedChain } from './chains.models.js';
import { EVMDLCInterface } from '../chains/ethereum/models/ethereum-dlc.interface.js';
export interface BlockchainInterface {
  chainName: PrefixedChain;
  startListening(): void;
  checkAndGetVault(vaultUUID: string): Promise<EVMDLCInterface | undefined>;
  setVaultStatusFunded(
    vaultUUID: string,
    bitcoinTransactionID: string
  ): Promise<TransactionReceipt | TxBroadcastResult>;
  setVaultStatusPostClosed(
    vaultUUID: string,
    bitcoinTransactionID: string
  ): Promise<TransactionReceipt | TxBroadcastResult>;
}
