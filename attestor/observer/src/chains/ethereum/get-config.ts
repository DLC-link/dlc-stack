import { ConfigSet } from '../../config/models.js';
import fetch from 'cross-fetch';
import { ethers } from 'ethers';
import { WebSocketProvider } from './utilities/websocket-provider.js';

async function fetchDeploymentInfo(subchain: string, version: string) {
  // TODO: versioning the deployment files
  const contract = 'DlcManager';
  try {
    const response = await fetch(
      `https://raw.githubusercontent.com/DLC-link/dlc-solidity/master/deploymentFiles/${subchain}/${contract}.json`
    );
    return await response.json();
  } catch (error) {
    throw new Error(`Could not fetch deployment info for ${contract} on ${subchain}`);
  }
}

export default async (config: ConfigSet) => {
  if (!config.apiKey) throw new Error(`API_KEY is required for ${config.chain}.`);
  switch (config.chain) {
    case 'ETH_MAINNET':
      return {
        provider: new WebSocketProvider(`wss://mainnet.infura.io/ws/v3/${config.apiKey}`),
        deploymentInfo: await fetchDeploymentInfo('mainnet', config.version),
      };
    case 'ETH_SEPOLIA':
      return {
        provider: new WebSocketProvider(`wss://sepolia.infura.io/ws/v3/${config.apiKey}`),
        deploymentInfo: await fetchDeploymentInfo('sepolia', config.version),
      };
    case 'ETH_GOERLI':
      return {
        provider: new WebSocketProvider(`wss://goerli.infura.io/ws/v3/${config.apiKey}`),
        deploymentInfo: await fetchDeploymentInfo('goerli', config.version),
      };
    case 'ETH_LOCAL':
      return {
        provider: new ethers.providers.JsonRpcProvider(`http://127.0.0.1:8545`),
        deploymentInfo: await fetchDeploymentInfo('localhost', config.version), // TODO:
      };
    default:
      break;
  }
};
