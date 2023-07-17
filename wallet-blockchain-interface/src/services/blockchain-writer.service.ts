import { Chain } from '../config/models.js';
import readEnvConfigs from '../config/read-env-configs.js';
import getETHConfig from '../chains/ethereum/get-config.js';
import { WrappedContract } from '../chains/shared/models/wrapped-contract.interface.js';
import { TransactionReceipt } from '@ethersproject/abstract-provider';

export default class BlockchainWriterService {
    private static blockchainWriter: BlockchainWriterService;

    private constructor() {}

    public static async getBlockchainWriter(): Promise<BlockchainWriterService> {
        if (!this.blockchainWriter) this.blockchainWriter = new BlockchainWriterService();
        return this.blockchainWriter;
    }

    public async getConfig(): Promise<WrappedContract> {
        let configSet = readEnvConfigs();

        switch (configSet.chain) {
            case 'ETH_MAINNET':
            case 'ETH_SEPOLIA':
            case 'ETH_GOERLI':
            case 'ETH_LOCAL':
                return await getETHConfig(configSet);
            case 'STACKS_MAINNET':
            case 'STACKS_TESTNET':
            case 'STACKS_MOCKNET':
            case 'STACKS_LOCAL':
            // return getStacksConfig(configSet);
            default:
                throw new Error(`${configSet.chain} is not a valid chain.`);
        }
    }

    public async setStatusFunded(uuid: string): Promise<TransactionReceipt> {
        const contractConfig = await this.getConfig();
        return await contractConfig.setStatusFunded(uuid);
    }

    public async postCloseDLC(uuid: string, btcTxId: string): Promise<TransactionReceipt> {
        const contractConfig = await this.getConfig();
        return await contractConfig.postCloseDLC(uuid, btcTxId);
    }

    public async getAllAttestors(): Promise<string[]> {
        const contractConfig = await this.getConfig();
        return await contractConfig.getAllAttestors();
    }
}
