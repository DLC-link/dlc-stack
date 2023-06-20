import { broadcastTransaction, makeContractCall, SignedContractCallOptions } from '@stacks/transactions';
import { config } from '../config/networks.config';
import { contracts } from '../config/contract-list';
import { AddStacks } from '../models/DTOs/add-stacks.dto';

export async function addNewDLC(params: AddStacks) {
  const contract = contracts.find((contr) => contr.contractName == params.sourceContract);
  if (!contract) throw `No such contract (${params.sourceContract}) found. Nothing was written on chain.`;

  const functionName = contract.functionNames.find((fn) => fn == 'post-create-dlc');
  if (!functionName) throw `Invalid function name. Nothing was written on chain.`;

  const txOptions: SignedContractCallOptions = {
    contractAddress: contract.deployerPrincipal,
    contractName: contract.contractName,
    functionName: functionName,
    functionArgs: contract.getAddNewTxArguments(params),
    senderKey: config.admin_private_key,
    validateWithAbi: true,
    network: config.network,
    fee: 100000,
    anchorMode: 1,
  };

  const transaction = await makeContractCall(txOptions);
  console.log('Transaction payload:', transaction.payload);
  const broadcastResponse = await broadcastTransaction(transaction, config.network);
  // console.log("Broadcast response: ", broadcastResponse);
  return broadcastResponse;
}
