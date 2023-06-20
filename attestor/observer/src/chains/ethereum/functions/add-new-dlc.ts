import { TransactionResponse } from '@ethersproject/abstract-provider';
import { AddEthereum } from '../models/DTOs/add-eth.dto';
import { getContracts } from '../config/contract-list';

export async function addNewDLCETH(params: AddEthereum) {
  const _contract = getContracts().find((contr) => contr.address == params.sourceContract);
  if (!_contract) throw `No such contract (${params.sourceContract}) found. Nothing was written on chain.`;

  const args = _contract.getAddNewArgs(params);

  try {
    const gasLimit = await _contract.contractWithSigner.estimateGas.postCreateDLC(...args);
    console.log('gasLimit estimation', gasLimit);

    const transaction: TransactionResponse = await _contract.contractWithSigner.postCreateDLC(...args, {
      gasLimit: gasLimit.add(10000),
      nonce: undefined,
    });
    transaction.wait();
    return transaction;
  } catch (error) {
    console.error(error);
    return error;
  }
}
