import dotenv from 'dotenv';
import fs from 'fs';
import yaml from 'js-yaml';

import { ChainConfig, validChains } from '../config/models.js';

// The yaml file should be in the following format:
interface NodeConfig {
    settings: {
        'solidity-branch': string;
        // 'storage-api-endpoint': string;
        'router-wallet-address': string;
        'public-server-port': number;
        'private-server-port': number;
        'tls-enabled'?: boolean;
    };
    'evm-chains': ChainConfig[];
    'stx-chains': ChainConfig[];
}

export default class ConfigService {
    private static config: NodeConfig;

    public static getConfig(): NodeConfig {
        if (!this.config) {
            this.config = this.readConfig();
        }
        return this.config;
    }

    private static readConfig(): NodeConfig {
        dotenv.config();
        try {
            const configFile = fs.readFileSync(process.env.CONFIG_LOCATION ?? './config.yaml', 'utf8');
            let config = yaml.load(configFile) as NodeConfig;

            let evmChainConfigs: ChainConfig[] = config['evm-chains'];

            if (!evmChainConfigs) {
                evmChainConfigs = [];
            } else {
                evmChainConfigs = evmChainConfigs.map((chainConfig) => {
                    this.validateNetwork(chainConfig);
                    chainConfig.version = chainConfig.version.toString();
                    chainConfig = this.validatePrivateKey(chainConfig);
                    return this.validateApiKey(chainConfig);
                });
            }

            let stxChainConfigs: ChainConfig[] = config['stx-chains'];

            if (!stxChainConfigs) {
                stxChainConfigs = [];
            } else {
                stxChainConfigs = stxChainConfigs.map((chainConfig) => {
                    this.validateNetwork(chainConfig);
                    chainConfig.version = chainConfig.version.toString();
                    chainConfig = this.validatePrivateKey(chainConfig);
                    return this.validateApiKey(chainConfig);
                });
            }

            config = { ...config, 'evm-chains': evmChainConfigs, 'stx-chains': stxChainConfigs };

            return config;
        } catch (error) {
            console.error(error);
            process.exit(1);
        }
    }

    public static getEvmChainConfigs(): ChainConfig[] {
        return this.getConfig()['evm-chains'];
    }

    public static getStxChainConfigs(): ChainConfig[] {
        return this.getConfig()['stx-chains'];
    }

    public static getSettings(): NodeConfig['settings'] {
        return this.getConfig().settings;
    }

    public static getEnv(key: string): string {
        const value = process.env[key];
        if (!value) throw new Error(`Environment variable ${key} is missing.`);
        return value;
    }

    private static validateNetwork(chainConfig: ChainConfig) {
        if (!validChains.includes(chainConfig.network)) {
            throw new Error(`CHAIN: ${chainConfig.network} is not a valid chain.`);
        }
    }

    private static validateApiKey(chainConfig: ChainConfig): ChainConfig {
        if (chainConfig.api_key && chainConfig.api_key.startsWith('${') && chainConfig.api_key.endsWith('}')) {
            const envVariable = chainConfig.api_key.slice(2, -1);
            chainConfig.api_key = this.getEnv(envVariable);
        }
        return chainConfig;
    }

    private static validatePrivateKey(chainConfig: ChainConfig): ChainConfig {
        if (
            chainConfig.private_key &&
            chainConfig.private_key.startsWith('${') &&
            chainConfig.private_key.endsWith('}')
        ) {
            const envVariable = chainConfig.private_key.slice(2, -1);
            chainConfig.private_key = this.getEnv(envVariable);
        }
        return chainConfig;
    }
}
