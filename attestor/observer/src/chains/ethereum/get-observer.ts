import { ethers } from 'ethers';
import { ChainConfig } from '../../config/models.js';
import getConfig from './get-network-config.js';
import { BlockchainInterface } from '../shared/models/observer.interface.js';
import { DlcManagerV1 } from './contracts/dlc-manager-v1.js';

export const getEthereumBlockchainInterface = async (config: ChainConfig): Promise<BlockchainInterface> => {
  const networkConfig = await getConfig(config);
  if (!networkConfig) throw new Error(`Could not load config for ${config.network}.`);

  console.log(`\n[${config.network}] Loaded config:`);
  console.dir(networkConfig.deploymentInfo, { depth: 1 });

  const deploymentInfo = networkConfig.deploymentInfo;
  const provider = networkConfig.provider;

  const wallet: ethers.Wallet = new ethers.Wallet(config.private_key, provider);

  const contract = new ethers.Contract(
    deploymentInfo.contract.address,
    deploymentInfo.contract.abi,
    provider
  ).connect(wallet);

  switch (config.version) {
    case '1':
      return DlcManagerV1(contract, deploymentInfo);
    default:
      throw new Error(`Version ${config.version} is not supported.`);
  }
};
