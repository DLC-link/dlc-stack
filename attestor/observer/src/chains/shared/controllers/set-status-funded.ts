import {
  addressToString,
  broadcastTransaction,
  callReadOnlyFunction,
  ContractPrincipal,
  contractPrincipalCV,
  cvToValue,
  makeContractCall,
  parsePrincipalString,
  SignedContractCallOptions,
} from '@stacks/transactions';
import { SetStatusFundedParams } from '../models/DTOs/set-status-funded.dto';
import { contracts as stacksContracts } from '../../stacks/config/contract-list';
import { config } from '../../stacks/config/networks.config';
import DLCInfoService, { DLCInfo } from '../../../services/dlc-info.service';
import { getContracts } from '../../ethereum/config/contract-list';
import { uuidToCV } from '../../../utilities/helper-functions';

async function handleEthereum(info: DLCInfo, params: SetStatusFundedParams) {
  const selectedContract = getContracts().find((contr) => contr.address == info.contractAddress);

  if (!selectedContract) throw `No contract associated with ${params.uuid}.`;

  try {
    const gasLimit = await selectedContract.contractWithSigner.estimateGas.setStatusFunded(params.uuid);
    const transaction = await selectedContract.contractWithSigner.setStatusFunded(params.uuid, {
      gasLimit: gasLimit.add(10000),
    });
    const txReceipt = await transaction.wait();

    console.log('Funded request Transaction receipt: ', txReceipt);
    return txReceipt.status === 1;
  } catch (error) {
    console.error(error);
    return false;
  }
}

async function handleStacks(info: DLCInfo, params: SetStatusFundedParams) {
  const selectedContract = stacksContracts.find((contr) => contr.contractFullName == info.contractAddress);

  if (!selectedContract) throw `No contract associated with ${params.uuid}.`;

  try {
    const functionName = 'get-callback-contract';
    const txOptions = {
      contractAddress: selectedContract.deployerPrincipal,
      contractName: selectedContract.contractName,
      functionName: functionName,
      functionArgs: [uuidToCV(params.uuid)],
      senderAddress: config.admin_address,
      network: config.network,
    };

    const transaction: any = await callReadOnlyFunction(txOptions);
    const callbackContract = cvToValue(transaction.value);
    console.log(`Callback contract for uuid: '${params.uuid}':`, callbackContract);

    const cbPrincipal = parsePrincipalString(callbackContract) as ContractPrincipal;

    const txOptions2: SignedContractCallOptions = {
      contractAddress: selectedContract.deployerPrincipal,
      contractName: selectedContract.contractName,
      functionName: 'set-status-funded',
      functionArgs: [
        uuidToCV(params.uuid),
        contractPrincipalCV(addressToString(cbPrincipal.address), cbPrincipal.contractName.content),
      ],
      senderKey: config.admin_private_key,
      validateWithAbi: true,
      network: config.network,
      fee: 100000,
      anchorMode: 1,
    };

    const transaction2 = await makeContractCall(txOptions2);
    console.log('Transaction payload:', transaction2.payload);
    const broadcastResponse = await broadcastTransaction(transaction2, config.network);
    console.log('Broadcast response: ', broadcastResponse);
    return true;
  } catch (error) {
    console.error(error);
    return false;
  }
}

export async function setStatusFunded(params: SetStatusFundedParams) {
  const dlcInfoSvc = DLCInfoService.getSvc();
  const info = dlcInfoSvc.DLCInfo.find((info) => info.uuid == params.uuid);
  if (!info)
    throw {
      type: 'dlcNotFound',
      message: `UUID '${params.uuid}' not found in any contracts. Nothing was written on chain.\n`,
    };

  if (info.fundedRequest || info.funded)
    throw {
      type: 'alreadyFunded',
      message: 'DLC funded status already requested/set',
    };

  dlcInfoSvc.addInfo({
    ...info,
    fundedRequest: true,
  });

  let funded = false;
  switch (info.chain) {
    case 'ETHEREUM': {
      funded = await handleEthereum(info, params);
      break;
    }

    case 'STACKS': {
      funded = await handleStacks(info, params);
      break;
    }
    default: {
      console.log('No implementation for /funded for this chain type.');
      break;
    }
  }
  dlcInfoSvc.addInfo({ ...info, fundedRequest: false, funded });
}
