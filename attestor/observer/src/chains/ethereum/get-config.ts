import { ConfigSet } from '../../config/models.js';
import fetch from 'cross-fetch';
import { ethers } from 'ethers';
import { WebSocketProvider } from './utilities/websocket-provider.js';
import { DeploymentInfo } from '../shared/models/deployment-info.interface.js';
import fs from 'fs';

async function fetchDeploymentInfo(subchain: string, version: string): Promise<DeploymentInfo> {
  const contract = 'DLCManager';
  const branch = process.env.BRANCH || 'master';
  console.log(`Fetching deployment info for ${contract} on ${subchain} from dlc-solidity/${branch}`);
  try {
    const response = await fetch(
      `https://raw.githubusercontent.com/DLC-link/dlc-solidity/${branch}/deploymentFiles/${subchain}/v${version}/${contract}.json`
    );
    return await response.json();
  } catch (error) {
    throw new Error(`Could not fetch deployment info for ${contract} on ${subchain}`);
  }
}

async function getLocalDeploymentInfo(path: string, contract: string, version: string): Promise<DeploymentInfo> {
  let dp = JSON.parse(fs.readFileSync(`${path}/v${version}/${contract}.json`, 'utf-8'));
  return dp;
}

export default async (
  config: ConfigSet
): Promise<
  | {
      provider: ethers.providers.JsonRpcProvider;
      deploymentInfo: DeploymentInfo;
    }
  | undefined
> => {
  switch (config.chain) {
    case 'ETH_MAINNET':
      if (!config.api_key) throw new Error(`API_KEY is required for ${config.chain}.`);
      return {
        provider: new WebSocketProvider(`wss://mainnet.infura.io/ws/v3/${config.api_key}`),
        deploymentInfo: await fetchDeploymentInfo('mainnet', config.version),
      };
    case 'ETH_SEPOLIA':
      if (!config.api_key) throw new Error(`API_KEY is required for ${config.chain}.`);
      return {
        provider: new WebSocketProvider(`wss://sepolia.infura.io/ws/v3/${config.api_key}`),
        deploymentInfo: await fetchDeploymentInfo('sepolia', config.version),
      };
    case 'ETH_GOERLI':
      if (!config.api_key) throw new Error(`API_KEY is required for ${config.chain}.`);
      return {
        provider: new WebSocketProvider(`wss://goerli.infura.io/ws/v3/${config.api_key}`),
        deploymentInfo: await fetchDeploymentInfo('goerli', config.version),
      };
    case 'ETH_LOCAL':
      return {
        provider: new ethers.providers.JsonRpcProvider(`http://127.0.0.1:8545`),
        deploymentInfo: await getLocalDeploymentInfo('./deploymentFiles/localhost', 'DlcManager', config.version), // TODO:
      };
    case 'OKX_TESTNET':
      return {
        provider: new WebSocketProvider('wss://x1testws.okx.com'),
        deploymentInfo: await fetchDeploymentInfo('X1test', config.version),
      };

    default:
      break;
  }
};
