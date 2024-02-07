import { StacksMainnet, StacksMocknet, StacksNetwork, StacksTestnet } from '@stacks/network';
import { Chain } from '../../../config/chains.models.js';

export function getStacksNetwork(network: Chain, endpoint: string): StacksNetwork {
  switch (network) {
    case 'mainnet':
        return new StacksMainnet();
    case 'testnet':
        return new StacksTestnet();
    case 'mocknet':
       return new StacksMocknet({
            url: `${endpoint}`,
        });
    case 'local':
        return  new StacksMocknet();
        break;
    default:
        throw new Error(`${network} is not a valid chain.`);
}
}
