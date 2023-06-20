import dotenv from 'dotenv';
dotenv.config();

import { abi } from '../chains/ethereum/config/ABIs/dlc-manager-v0.abi';
import { abi as abiNFT } from '../chains/ethereum/config/ABIs/btc-nft.abi';

const ETHNetworks = (process.env.ETH_NETWORKS as string).split(',');

const localETHConfig = {
  dlcManager: {
    address: process.env.ETH_MAN_V0_ADDRESS as string,
    abi: abi,
  },
  nftContract: {
    address: process.env.NFT_CONTRACT_ADDRESS as string,
    abi: abiNFT,
  },
};

export const config = {
  stacksEnabled: process.env.STACKS_ENABLED === 'true',
  stacksNetwork: process.env.STACKS_NETWORK as string,
  ethEnabled: process.env.ETH_ENABLED === 'true',
  ETHNetworks: ETHNetworks,
  verboseLogs: process.env.VERBOSE_LOGS === 'true',
  txHandlingEnabled: !(process.env.TX_HANDLING_DISABLED === 'true'),
  localETHConfig: localETHConfig,
};
