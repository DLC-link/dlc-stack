import { EthereumContract } from '../models/classes/contract';
import { DLCManagerV0 } from './contracts/dlc-manager-v0';
import { config } from '../../../utilities/config';

const networks = config.ETHNetworks;
const contracts: EthereumContract[] = [];

if (config.ethEnabled) {
  if (networks.includes('goerli')) {
    const contract = new DLCManagerV0('ETHEREUM', 'goerli', 'DlcManager');
    await contract.init();
    contracts.push(contract);
  }
  if (networks.includes('sepolia')) {
    const contract = new DLCManagerV0('ETHEREUM', 'sepolia', 'DlcManager');
    await contract.init();
    contracts.push(contract);
  }
  if (networks.includes('localhost')) {
    // This approach tries to pull in the contract from the local deployment files
    try {
      const contract = new DLCManagerV0('ETHEREUM', 'localhost', 'DlcManager');
      await contract.init(true);
      contracts.push(contract);
    } catch (err) {
      console.log('Error initializing localhost contract', err);
    }

    // This approach sets the contract address and ABI manually (in .env)
    // try {
    //   const contract = new DLCManagerV0(
    //     'ETHEREUM',
    //     'localhost',
    //     'DlcManager',
    //     {
    //       address: config.localETHConfig.dlcManager.address,
    //       abi: config.localETHConfig.dlcManager.abi,
    //     },
    //     {
    //       address: config.localETHConfig.nftContract.address,
    //       abi: config.localETHConfig.nftContract.abi,
    //     }
    //   );
    //   await contract.init(true);
    //   contracts.push(contract);
    // } catch (err) {
    //   console.log('Error initializing local contract', err);
    // }
  }
  if (networks.includes('mainnet')) {
  }
}

export function getContracts() {
  return contracts;
}
