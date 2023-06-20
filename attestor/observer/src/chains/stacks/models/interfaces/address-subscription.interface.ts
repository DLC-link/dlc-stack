import { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';

export interface AddressSubscription {
  address: string;
  subscription: { unsubscribe: () => void };
  handleTx: (tx: ContractCallTransaction) => void;
}
