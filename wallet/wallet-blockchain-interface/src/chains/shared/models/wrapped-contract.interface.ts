import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { TxBroadcastResult } from '@stacks/transactions';

export interface WrappedContract {
    setStatusFunded: (uuid: string, btcTxId: string) => Promise<TransactionReceipt> | Promise<TxBroadcastResult>;
    postCloseDLC: (uuid: string, btcTxId: string) => Promise<TransactionReceipt> | Promise<TxBroadcastResult>;
    getDLCInfo: (uuid: string) => Promise<any>;
}
