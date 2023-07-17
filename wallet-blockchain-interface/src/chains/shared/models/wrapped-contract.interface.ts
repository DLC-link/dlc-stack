import { TransactionReceipt } from '@ethersproject/abstract-provider';

export interface WrappedContract {
    setStatusFunded: (uuid: string) => Promise<TransactionReceipt>;
    postCloseDLC: (uuid: string, btcTxId: string) => Promise<TransactionReceipt>;
    getAllAttestors: () => Promise<string[]>;
}
