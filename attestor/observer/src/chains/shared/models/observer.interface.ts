import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { TxBroadcastResult } from '@stacks/transactions';
import { PrefixedChain } from '../../../config/models.js';
import { EVMDLCInterface } from '../../ethereum/models/dlc-info.interface.js';
export interface BlockchainInterface {
  chainName: PrefixedChain;
  startListening(): void;
  getAllVaults(): Promise<any>;
  getDLCInfo(vaultUUID: string): Promise<EVMDLCInterface | undefined>;
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
