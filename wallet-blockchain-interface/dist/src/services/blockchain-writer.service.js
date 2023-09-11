import readEnvConfigs from '../config/read-env-configs.js';
import getETHConfig from '../chains/ethereum/get-config.js';
import getStacksConfig from '../chains/stacks/get-config.js';
export default class BlockchainWriterService {
    static blockchainWriter;
    static contractConfig;
    constructor() { }
    static async getBlockchainWriter() {
        if (!this.blockchainWriter)
            this.blockchainWriter = new BlockchainWriterService();
        return this.blockchainWriter;
    }
    async readConfig() {
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
                return await getStacksConfig(configSet);
            default:
                throw new Error(`${configSet.chain} is not a valid chain.`);
        }
    }
    async getWrappedContract() {
        if (!BlockchainWriterService.contractConfig) {
            BlockchainWriterService.contractConfig = await this.readConfig();
        }
        return BlockchainWriterService.contractConfig;
    }
    async setStatusFunded(uuid) {
        const contractConfig = await this.getWrappedContract();
        return await contractConfig.setStatusFunded(uuid);
    }
    async postCloseDLC(uuid, btcTxId) {
        const contractConfig = await this.getWrappedContract();
        return await contractConfig.postCloseDLC(uuid, btcTxId);
    }
    async getAllAttestors() {
        const contractConfig = await this.getWrappedContract();
        return await contractConfig.getAllAttestors();
    }
}
