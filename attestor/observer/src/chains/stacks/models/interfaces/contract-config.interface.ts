import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import { ClarityValue } from '@stacks/transactions';
import { Chain } from '../../../chain-types.interface';
import { AddStacks } from '../DTOs/add-stacks.dto';
import { FunctionName } from './function-names.type';

export interface ContractConfig {
  chainType: Chain;
  deployerPrincipal: string;
  contractName: string;
  contractFullName: string;
  dlcNFTName: string;
  registeredContractNFTName: string;
  functionNames: Array<FunctionName>;
  handleTx: (txInfo: ContractCallTransaction) => void;
  getAddNewTxArguments: (params: AddStacks) => Array<ClarityValue>;
}
