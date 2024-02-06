import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { TxBroadcastResult } from '@stacks/transactions';
export interface BlockchainInterface {
  startListening(): void;
  getAllVaults(): Promise<any>;
  setVaultStatusFunded(vaultUUID: string, bitcoinTransactionID: string): Promise<TransactionReceipt | TxBroadcastResult>;
  setVaultStatusPostClosed(vaultUUID: string, bitcoinTransactionID: string):  Promise<TransactionReceipt | TxBroadcastResult>;
}
