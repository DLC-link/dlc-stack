import { Contract, ContractInterface } from 'ethers';
import { Chain, ETHSubchain } from '../../../chain-types.interface';
import { AddEthereum } from '../DTOs/add-eth.dto';
import { PostMintEthereum } from '../DTOs/post-mint-eth.dto';

export interface EthereumContractConfig {
  address: string;
  abi: ContractInterface;
  chain: Chain;
  subchain: ETHSubchain | undefined;
  handleTx: (contract: Contract, nftContractAddress: string) => void;
  getAddNewArgs: (params: AddEthereum) => Array<any>;
}
