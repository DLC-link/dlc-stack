import { TransactionResponse } from '@ethersproject/abstract-provider';
import { PostMintEthereum } from '../models/DTOs/post-mint-eth.dto';
import { getContracts } from '../config/contract-list';

export async function postMintNft(params: PostMintEthereum) {
  const _contract = getContracts().find((contr) => contr.address == params.sourceContract);
  if (!_contract) throw `No such contract (${params.sourceContract}) found. Nothing was written on chain.`;

  const transaction: TransactionResponse = await _contract.contractWithSigner.postMintBtcNft(
    params.uuid,
    params.nftId,
    {
      gasLimit: 500000,
      nonce: undefined,
    }
  );
  transaction.wait();
  return transaction;
}
