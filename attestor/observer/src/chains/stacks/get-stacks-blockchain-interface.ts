import { getAddressFromPrivateKey } from '@stacks/transactions';
import { BlockchainInterface } from '../../config/blockchain-interface.interface.js';
import { ChainConfig } from '../../config/chains.models.js';
import { DlcManagerV1 } from './contracts/dlc-manager-v1.js';
import { NetworkConfig } from './models/interfaces.js';
import { getStacksNetwork } from './utilities/get-stacks-network.js';
import getStacksNetworkConfig from './get-stacks-network-config.js';

export const getStackBlockchainInterface= async (config: ChainConfig): Promise<BlockchainInterface> => {
  const networkConfig: NetworkConfig = await getStacksNetworkConfig(config);
  if (!networkConfig) throw new Error(`Could not load config for ${config.network}.`);

  console.log(`\n[${config.network}] Loaded config:`);
  console.dir(networkConfig.deploymentInfo, { depth: 1 });

  const deploymentInfo = networkConfig.deploymentInfo;
  const socket = networkConfig.socket;

  const stacksNetwork = getStacksNetwork(config.network, config.endpoint);
  const wallet = { privateKey: config.private_key, address: getAddressFromPrivateKey(config.private_key, stacksNetwork.version) };

  switch (config.version) {
    case '1':
      return DlcManagerV1(socket, deploymentInfo, stacksNetwork, wallet);
    default:
      throw new Error(`Version ${config.version} is not supported.`);
  }
};
