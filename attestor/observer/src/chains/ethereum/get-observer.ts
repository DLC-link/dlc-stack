import { ethers } from 'ethers';
import { ConfigSet } from '../../config/models.js';
import loadConfig from './get-config.js';
import { DeploymentInfo } from '../shared/models/deployment-info.interface.js';
import { Observer } from '../shared/models/observer.interface.js';
import { DlcManagerV0 } from './contracts/dlc-manager-v0.js';

export const getEthObserver = async (config: ConfigSet): Promise<Observer> => {
  const networkConfig = await loadConfig(config);
  if (!networkConfig) throw new Error(`Could not load config for ${config.chain}.`);

  const deploymentInfo = networkConfig.deploymentInfo as DeploymentInfo;
  const contract = new ethers.Contract(
    deploymentInfo.contract.address,
    deploymentInfo.contract.abi,
    networkConfig.provider
  );

  switch (config.version) {
    case '0':
      return DlcManagerV0(contract, deploymentInfo);
    default:
      throw new Error(`Version ${config.version} is not supported.`);
  }
};
