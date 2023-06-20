import {
  makeContractNonFungiblePostCondition,
  NonFungibleConditionCode,
  createAssetInfo,
  addressToString,
  contractPrincipalCV,
  parsePrincipalString,
  ContractPrincipal,
  broadcastTransaction,
  makeContractCall,
  uintCV,
  bufferCV,
} from '@stacks/transactions';
import { TransactionResponse } from '@ethersproject/abstract-provider';
import { hexToBytes } from '../../../utilities/helper-functions';
import { closeRequestDirect } from './close-request-direct';
import { config } from '../../stacks/config/networks.config';
import DLCInfoService from '../../../services/dlc-info.service';
import { getContracts } from '../../ethereum/config/contract-list';
import { PostCloseOutcomeDTO } from '../models/DTOs/post-close-outcome.dto';

// Fetching Oracle Attestation and then calling post-close-dlc with the outcome we got back.
// This allows us to check it against what was stored on-chain.
export async function callPostCloseOutcome(data: PostCloseOutcomeDTO, precisionShift: number = 0) {
  // NOTE: this whole flow
  const formatOutcome = (value: number) => Math.round(value / 10 ** precisionShift);
  const formattedOutcome = formatOutcome(data.outcome);

  // Fetching Oracle Attestation:
  const oracleResponse = await closeRequestDirect({
    uuid: data.uuid,
    outcome: formattedOutcome,
  });
  if (!oracleResponse) throw 'No Oracle Response';

  const dlcInfoSvc = DLCInfoService.getSvc();
  const info = dlcInfoSvc.DLCInfo.find((info) => info.uuid == data.uuid);
  if (!info) throw `UUID '${data.uuid}' not found in any contracts. Nothing was written on chain.\n`;
  if (info.outcome === undefined || info.outcome === null) throw `DLC ${data.uuid} has no outcome saved\n`;

  console.log('coming from chain:', data.outcome, 'formatted: ', formattedOutcome, 'observer info:', info.outcome);

  // Switching outcome to the response one, before sending it back on chain:
  // TODO: This value shouldn't be trusted. Instead, we should decode the actual attestation hex.
  const oracleOutcome = oracleResponse.outcome;

  if (oracleOutcome != formatOutcome(info.outcome))
    throw `Oracle and stored outcomes are different. Oracle: ${oracleOutcome}, Info: ${info.outcome}`;
  const responseOutcome = info.outcome;

  switch (info.chain) {
    case 'ETHEREUM':
      const selectedContract = getContracts().find((contr) => contr.address == info.contractAddress);
      if (!selectedContract) throw `No ETH contract associated with ${data.uuid}`;

      try {
        const gasLimit = await selectedContract.contractWithSigner.estimateGas.postCloseDLC(data.uuid, responseOutcome);
        // NOTE: this must be called by ADMIN_ROLE, the observer must have that.
        const tx: TransactionResponse = await selectedContract.contractWithSigner.postCloseDLC(
          data.uuid,
          responseOutcome,
          {
            gasLimit: gasLimit.add(10000),
            nonce: undefined,
          }
        );
        console.log('Transaction payload:', await tx.wait());
      } catch (error) {
        console.error(error);
      }
      break;

    case 'STACKS':
      if (!data.assetInfo || !data.callbackContract) throw 'Missing data in postCloseOutcome request';
      const callbackContractPrincipal = parsePrincipalString(data.callbackContract) as ContractPrincipal;
      const functionName = 'post-close-dlc';
      const contractNonFungiblePostCondition = makeContractNonFungiblePostCondition(
        data.assetInfo.contractAddress,
        data.assetInfo.contractName,
        NonFungibleConditionCode.Sends,
        createAssetInfo(data.assetInfo.contractAddress, data.assetInfo.contractName, data.assetInfo.NFTName),
        bufferCV(hexToBytes(data.uuid))
      );

      function populateTxOptions() {
        if (!data.assetInfo || !data.callbackContract) throw 'Missing data in postCloseOutcome request';
        return {
          contractAddress: data.assetInfo.contractAddress,
          contractName: data.assetInfo.contractName,
          functionName: functionName,
          functionArgs: [
            bufferCV(hexToBytes(data.uuid)),
            contractPrincipalCV(
              addressToString(callbackContractPrincipal.address),
              callbackContractPrincipal.contractName.content
            ),
            uintCV(responseOutcome),
          ],
          postConditions: [contractNonFungiblePostCondition],
          senderKey: config.admin_private_key,
          validateWithAbi: true,
          network: config.network,
          fee: 100000, //0.1STX
          anchorMode: 1,
        };
      }

      const transaction = await makeContractCall(populateTxOptions());
      console.log('Transaction payload:', transaction.payload);
      const broadcastResponse = await broadcastTransaction(transaction, config.network);
      console.log('Broadcast response: ', broadcastResponse);

      break;
    default:
      break;
  }
}
