import readEnvConfigs from '../config/read-env-configs.js';
import { getNonce } from '@stacks/transactions';
import getNetworkConfig from '../chains/stacks/get-network-config.js';

// Stacks requires us to do some magic regarding transaction nonces.
// https://github.com/stacks-network/stacks-blockchain/issues/2376
export default class StacksNonceService {
    private static nonce: number;

    private constructor() {}

    public static async getNonce(): Promise<number> {
        const blockChainNonce = await this.getNonceFromBlockchain();
        if (!this.nonce || blockChainNonce > this.nonce) {
            console.log(`[StacksNonceService] Syncing nonce from blockchain to ${blockChainNonce}...`);
            this.nonce = blockChainNonce;
        } else {
            this.nonce++;
            console.log(`[StacksNonceService] Nonce: ${this.nonce}`);
        }
        return this.nonce;
    }

    private static async getNonceFromBlockchain(): Promise<number> {
        let { network, walletAddress } = await getNetworkConfig(readEnvConfigs());
        return Number(await getNonce(walletAddress, network));
    }
}
