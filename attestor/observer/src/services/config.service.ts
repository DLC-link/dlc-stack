import dotenv from 'dotenv';
import fs from 'fs';
import yaml from 'js-yaml';

import { ChainConfig, validChains } from '../config/models.js';
import { defaultConfigs } from '../config/network-configs.js';

// The yaml file should be in the following format:
interface NodeConfig {
  settings: {
    'solidity-branch': string;
    'storage-api-endpoint': string;
    'dev-endpoints-enabled'?: boolean;
    'mocknet-address'?: string;
  };
  chains: ChainConfig[];
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
      const configFile = fs.readFileSync('./config.yaml', 'utf8');
      const config = yaml.load(configFile) as NodeConfig;

      let chainConfigs: ChainConfig[] = config.chains;

      chainConfigs.forEach((chainConfig) => {
        this.validateNetwork(chainConfig);
        const defaultConfig = defaultConfigs.find((defaultConfig) => defaultConfig.network === chainConfig.network);
        this.validateApiKey(chainConfig, defaultConfig);
      });

      return config;
    } catch (error) {
      console.error(error);
      process.exit(1);
    }
  }

  public static getChainConfigs(): ChainConfig[] {
    const rawConfigs = this.getConfig().chains;
    let results;
    results = rawConfigs.map((chainConfig) => {
      const defaultConfig = defaultConfigs.find((defaultConfig) => defaultConfig.network === chainConfig.network);
      if (defaultConfig) {
        chainConfig.version = chainConfig.version || defaultConfig.version;
        chainConfig.endpoint = chainConfig.endpoint || defaultConfig.endpoint;
        chainConfig.name = chainConfig.name || defaultConfig.name;
        chainConfig.type = chainConfig.type || defaultConfig.type;
      }
      return chainConfig;
    });
    return results;
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

  private static validateApiKey(chainConfig: ChainConfig, defaultConfig?: ChainConfig) {
    if (chainConfig.api_key && chainConfig.api_key.startsWith('${') && chainConfig.api_key.endsWith('}')) {
      const envVariable = chainConfig.api_key.slice(2, -1);
      if (!process.env[envVariable]) {
        throw new Error(`Environment variable ${envVariable} is required but not found.`);
      }
      chainConfig.api_key = process.env[envVariable];
    }
    chainConfig.api_key_required = chainConfig.api_key_required ?? defaultConfig?.api_key_required;
    if (
      (!chainConfig.endpoint || chainConfig.endpoint == defaultConfig?.endpoint) &&
      defaultConfig?.api_key_required &&
      (!chainConfig.api_key_required || !chainConfig.api_key)
    ) {
      throw new Error(`API key is required when using the default endpoint for ${chainConfig.network}.`);
    }
  }
}
