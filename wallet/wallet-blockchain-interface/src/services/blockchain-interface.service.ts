import getETHConfig from '../chains/ethereum/get-config.js';
import getStacksConfig from '../chains/stacks/get-config.js';
import { WrappedContract } from '../chains/shared/models/wrapped-contract.interface.js';
import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { TxBroadcastResult } from '@stacks/transactions';
import { getAttestors } from '../config/attestor-lists.js';
import ConfigService from './config.service.js';
import { evmPrefix, stxPrefix } from '../config/models.js';
import RouterWalletService from './router-wallet.service.js';

export default class BlockchainInterfaceService {
    private static blockchainWriter: BlockchainInterfaceService;

    private constructor() {}

    public static async getBlockchainWriter(): Promise<BlockchainInterfaceService> {
        if (!this.blockchainWriter) this.blockchainWriter = new BlockchainInterfaceService();
        return this.blockchainWriter;
    }

    public async getWrappedContract(chain: string): Promise<WrappedContract> {
        if (chain.startsWith('"')) chain = chain.slice(1, -1);

        const config = ConfigService.getConfig();
        const chainSplit = chain.split('-');

        switch (`${chainSplit[0]}-`) {
            case evmPrefix:
                if (!config['evm-chains']) throw new Error(`[WBI] No evm-chains found in config.`);
                const contractConfig = config['evm-chains'].find((config) => config.network == `${chainSplit[1]}`);
                if (!contractConfig) throw new Error(`[WBI] No config found for chain ${chain}`);
                return await getETHConfig(contractConfig);

            case stxPrefix:
                if (!config['stx-chains']) throw new Error(`[WBI] No stx-chains found in config.`);
                const stxConfig = config['stx-chains'].find((config) => config.network == `${chainSplit[1]}`);
                if (!stxConfig) throw new Error(`[WBI] No config found for chain ${chain}`);
                return await getStacksConfig(stxConfig);

            default:
                throw new Error(`[WBI] ${chain} is not a valid chain.`);
        }
    }

    public async setStatusFunded(
        uuid: string,
        btcTxId: string,
        chain: string
    ): Promise<TransactionReceipt | TxBroadcastResult> {
        const contractConfig = await this.getWrappedContract(chain);
        return await contractConfig.setStatusFunded(uuid, btcTxId);
    }

    public async postCloseDLC(
        uuid: string,
        btcTxId: string,
        chain: string
    ): Promise<TransactionReceipt | TxBroadcastResult> {
        const contractConfig = await this.getWrappedContract(chain);
        return await contractConfig.postCloseDLC(uuid, btcTxId);
    }

    public async getDLCInfo(uuid: string): Promise<any> {
        let res = await (await RouterWalletService.getRouterWallet()).getChainForUUID(uuid);
        let chain = await res.json();
        // if (!chain) {
        //     console.log(`[WBI] Chain not found for UUID ${uuid}. Looking up chain from attestors.`);
        //     chain = (await (await fetch(`${getAttestors()[0]}/event/${uuid}`)).json())['chain'];
        // }
        if (!chain) throw new Error(`Could not find chain for UUID ${uuid}`);
        console.log(`[WBI] Chain found for UUID ${uuid}: ${chain}`);
        const contractConfig = await this.getWrappedContract(chain);
        return await contractConfig.getDLCInfo(uuid);
    }
}
