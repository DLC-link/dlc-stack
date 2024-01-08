import { ChainConfig } from '../../config/models.js';
import { DlcManagerV1 } from './config/contract-configs/dlc-manager-v1.js';
import { getConfig } from './config/get-config.js';
import { Observer } from '../shared/models/observer.interface.js';
import { ContractConfig, NetworkConfig } from './models/interfaces.js';
import { fetchTXInfo } from './utilities/api-calls.js';
import { AddressTransactionWithTransfers } from '@stacks/blockchain-api-client';
import { Transaction } from '@stacks/stacks-blockchain-api-types';

export default async (config: ChainConfig): Promise<Observer> => {
  let dlcManager: ContractConfig;
  const networkConfig: NetworkConfig = getConfig(config);

  switch (config.version) {
    case '1':
      dlcManager = new DlcManagerV1(networkConfig.socket, networkConfig.deploymentInfo);
      await dlcManager.init();
      break;
  }

  return {
    start: () => {
      networkConfig.socket.socket.on(
        'address-transaction',
        async (address: string, txWithTransfers: AddressTransactionWithTransfers) => {
          try {
            const tx = txWithTransfers.tx as Transaction;
            if (tx.tx_status !== 'success') {
              console.log(`[Stacks] Skip - Failed tx: ${tx.tx_id}`);
              return;
            }
            if (tx.is_unanchored) {
              console.log(`[Stacks] Skip - Microblock tx: ${tx.tx_id}`);
              return;
            }
            const txInfo = await fetchTXInfo(tx.tx_id, networkConfig.deploymentInfo.api_base_extended);
            if (txInfo.event_count < 1) {
              console.log(`[Stacks] Skip - Non-printing tx: ${tx.tx_id}`);
              return;
            }
            if (dlcManager.checkAddresses(address)) {
              await dlcManager.handleTx(txInfo);
            }
          } catch (error) {
            console.error(error);
          }
        }
      );
    },
  };
};
