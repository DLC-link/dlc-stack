import { getEthObserver } from '../chains/ethereum/get-observer.js';
import { Observer } from '../chains/shared/models/observer.interface.js';
import getStacksObserver from '../chains/stacks/get-observer.js';
import readConfig from './read-env-configs.js';

export default async (): Promise<Observer[]> => {
  const config = readConfig();

  const observerPromises = config.map((configSet, index) => {
    switch (configSet.chain) {
      case 'ETH_MAINNET':
      case 'ETH_SEPOLIA':
      case 'ETH_GOERLI':
      case 'ETH_LOCAL':
        return getEthObserver(configSet);
      case 'STACKS_MAINNET':
      case 'STACKS_TESTNET':
      case 'STACKS_MOCKNET':
      case 'STACKS_LOCAL':
        return getStacksObserver(configSet);
      default:
        throw new Error(`CHAIN_${index}: ${configSet.chain} is not a valid chain.`);
    }
  });

  const observers = await Promise.all(observerPromises);
  return observers;
};
