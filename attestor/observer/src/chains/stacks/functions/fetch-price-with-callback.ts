import {
  parsePrincipalString,
  ContractPrincipal,
  bufferCVFromString,
  tupleCV,
  uintCV,
  listCV,
  bufferCV,
  contractPrincipalCV,
  addressToString,
  makeContractCall,
  broadcastTransaction,
} from '@stacks/transactions';
import redstone from 'redstone-api-extended';

import { liteSignatureToStacksSignature, uuidToCV } from '../../../utilities/helper-functions';
import { config } from '../config/networks.config';
import { CloseInternalCallbackDTO } from '../models/DTOs/close-internal-callback.dto';

export async function callFetchPriceWithCallback(data: CloseInternalCallbackDTO) {
  console.log('Fetching price data for:', data.uuid);

  const callbackContractPrincipal = parsePrincipalString(data.callbackContract) as ContractPrincipal;
  const functionName = data.functionName;

  function populateTxOptions(price: number, timestamp: number, signature: string, symbol: string) {
    const liteSig = liteSignatureToStacksSignature(signature);
    const sig = Buffer.from(liteSig);
    const tupCV = tupleCV({
      symbol: bufferCVFromString(symbol),
      value: uintCV(price),
    });

    return {
      contractAddress: data.assetInfo.contractAddress,
      contractName: data.assetInfo.contractName,
      functionName: functionName,
      functionArgs: [
        uuidToCV(data.uuid),
        uintCV(timestamp),
        listCV([tupCV]),
        bufferCV(sig),
        contractPrincipalCV(
          addressToString(callbackContractPrincipal.address),
          callbackContractPrincipal.contractName.content
        ),
      ],
      senderKey: config.admin_private_key,
      validateWithAbi: true,
      network: config.network,
      fee: 100000, //0.1STX
      anchorMode: 1,
    };
  }

  // TODO: This might be temporary
  const asset = 'BTC';

  const dataPackage = await redstone.oracle.getFromDataFeed('redstone', asset);
  console.log('Redstone price package:', dataPackage);

  const liteEvmSignature = dataPackage.liteSignature;
  const symbol = asset;
  const price = dataPackage.priceData.values[0];
  const timestamp = dataPackage.priceData.timestamp;

  const transaction = await makeContractCall(populateTxOptions(price, timestamp, liteEvmSignature, symbol));
  console.log('Transaction payload:', transaction.payload);
  const broadcastResponse = await broadcastTransaction(transaction, config.network);
  console.log('Broadcast response: ', broadcastResponse);
}
