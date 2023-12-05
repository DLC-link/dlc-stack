import { getAddressFromPrivateKey } from '@stacks/transactions';
import { Chain, ChainConfig } from '../../config/models.js';
import { StacksMainnet, StacksMocknet, StacksNetwork, StacksTestnet } from '@stacks/network';

let networkConfigMap: Map<
    Chain,
    { stacksNetwork: StacksNetwork; deployer: string; api_base_extended: string; walletAddress: string }
> = new Map();

export default async (
    config: ChainConfig
): Promise<{ stacksNetwork: StacksNetwork; deployer: string; api_base_extended: string; walletAddress: string }> => {
    // we don't want to create a new network config for every request
    const existingNetworkConfig = networkConfigMap.get(config.network);
    if (existingNetworkConfig) {
        return existingNetworkConfig;
    }

    let stacksNetwork: StacksNetwork;
    const { deployer, private_key, endpoint } = config;
    if (!deployer) throw new Error(`[Stacks] No deployer address found in config.`);
    const api_base_extended = `${endpoint}/extended/v1`;

    switch (config.network) {
        case 'mainnet':
            stacksNetwork = new StacksMainnet();
            break;
        case 'testnet':
            stacksNetwork = new StacksTestnet();
            break;
        case 'mocknet':
            stacksNetwork = new StacksMocknet({
                url: `${endpoint}`,
            });
            break;
        case 'local':
            stacksNetwork = new StacksMocknet();
            break;
        default:
            throw new Error(`${config.network} is not a valid chain.`);
    }

    const walletAddress = getAddressFromPrivateKey(private_key, stacksNetwork.version);
    const networkConfig = { stacksNetwork, deployer, api_base_extended, walletAddress };
    networkConfigMap.set(config.network, networkConfig);
    return networkConfig;
};
