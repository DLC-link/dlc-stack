import { getAddressFromPrivateKey } from '@stacks/transactions';
import { StacksMainnet, StacksMocknet, StacksTestnet } from '@stacks/network';
import { getEnv } from '../../config/read-env-configs.js';
let networkConfig;
export default async (config) => {
    if (networkConfig)
        return networkConfig;
    const walletKey = getEnv('PRIVATE_KEY');
    let api_base_extended;
    let network;
    let deployer;
    switch (config.chain) {
        case 'STACKS_MAINNET':
            network = new StacksMainnet();
            deployer = '';
            api_base_extended = 'https://api.hiro.so/extended/v1';
            break;
        case 'STACKS_TESTNET':
            network = new StacksTestnet();
            deployer = 'ST1JHQ5GPQT249ZWG6V4AWETQW5DYA5RHJB0JSMQ3';
            api_base_extended = 'https://api.testnet.hiro.so/extended/v1';
            break;
        case 'STACKS_MOCKNET':
            network = new StacksMocknet({
                url: `https://${getEnv('MOCKNET_ADDRESS')}`,
            });
            deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
            api_base_extended = `https://${getEnv('MOCKNET_ADDRESS')}/extended/v1`;
            break;
        case 'STACKS_LOCAL':
            network = new StacksMocknet();
            deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
            api_base_extended = 'http://localhost:3999/extended/v1';
            break;
        default:
            throw new Error(`${config.chain} is not a valid chain.`);
    }
    const walletAddress = getAddressFromPrivateKey(walletKey, network.version);
    networkConfig = { network, deployer, api_base_extended, walletAddress };
    return networkConfig;
};
