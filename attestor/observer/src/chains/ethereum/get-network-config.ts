import { ChainConfig } from '../../config/models.js';
import fetch from 'cross-fetch';
import { ethers } from 'ethers';
import { WebSocketProvider } from './utilities/websocket-provider.js';
import { DeploymentInfo } from '../shared/models/deployment-info.interface.js';
import fs from 'fs';
import ConfigService from '../../services/config.service.js';

async function fetchDeploymentInfo(subchain: string, version: string): Promise<DeploymentInfo> {
  const contract = 'DLCManager';
  const branch = ConfigService.getSettings()['solidity-branch'] || 'master';
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
  config: ChainConfig
): Promise<{
  provider: ethers.providers.JsonRpcProvider | WebSocketProvider;
  deploymentInfo: DeploymentInfo;
}> => {
  const deploymentInfo =
    config.network === 'localhost'
      ? await getLocalDeploymentInfo('./observer/deploymentFiles/localhost', 'DLCManager', config.version)
      : await fetchDeploymentInfo(config.network, config.version);

  let provider;
  if (config.endpoint?.startsWith('http')) {
    console.log(`Connecting to ${config.endpoint}`);
    provider = new ethers.providers.JsonRpcProvider(config.endpoint);
  } else if (config.endpoint?.startsWith('ws')) {
    provider = new WebSocketProvider(`${config.endpoint}${config.api_key ?? ''}`);
  } else {
    throw new Error(`Invalid endpoint ${config.endpoint}`);
  }

  return { provider, deploymentInfo };
};
