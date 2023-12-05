import { getNonce } from '@stacks/transactions';
import { StacksNetwork } from '@stacks/network';

// Stacks requires us to do some magic regarding transaction nonces.
// https://github.com/stacks-network/stacks-blockchain/issues/2376
export default class StacksNonceService {
    private static nonce: number;

    private constructor() {}

    public static async getNonce(stacksNetwork: StacksNetwork, walletAddress: string): Promise<number> {
        const blockChainNonce = await this.getNonceFromBlockchain(stacksNetwork, walletAddress);
        if (!this.nonce || blockChainNonce > this.nonce) {
            console.log(`[StacksNonceService] Syncing nonce from blockchain to ${blockChainNonce}...`);
            this.nonce = blockChainNonce;
        } else {
            this.nonce++;
            console.log(`[StacksNonceService] Nonce: ${this.nonce}`);
        }
        return this.nonce;
    }

    private static async getNonceFromBlockchain(stacksNetwork: StacksNetwork, walletAddress: string): Promise<number> {
        return Number(await getNonce(walletAddress, stacksNetwork));
    }
}
