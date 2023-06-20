import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import {
  addressToString,
  bufferCV,
  ClarityValue,
  ContractPrincipal,
  contractPrincipalCV,
  parsePrincipalString,
  uintCV,
  standardPrincipalCV,
} from '@stacks/transactions';
import { ContractConfig } from '../../models/interfaces/contract-config.interface';
import { config } from '../networks.config';
import { hexToBytes, timestampToDate } from '../../../../utilities/helper-functions';
import unwrapper from '../../functions/unwrappers';
import { AddStacksDirectDTO } from '../../models/DTOs/add-stacks.dto';
import { FunctionName } from '../../models/interfaces/function-names.type';
// import loggerSvc from '../../../../services/logger.service';
import ContractRegistrationService from '../../services/contract-registration.service';
import DLCInfoService from '../../../../services/dlc-info.service';
import { Chain } from '../../../chain-types.interface';
import { createRequestDirect } from '../../../shared/functions/create-request-direct';
import { callPostCloseOutcome } from '../../../shared/functions/post-close-outcome';
import { callFetchPriceWithCallback } from '../../functions/fetch-price-with-callback';

const registrationSvc = ContractRegistrationService.getSvc();
const dlcInfoSvc = DLCInfoService.getSvc();

const contractName = 'dlc-manager-priced-v0-1';
const functionNames: Array<FunctionName> = [
  'create-dlc',
  'post-create-dlc',
  'close-dlc',
  'post-close-dlc',
  'get-btc-price',
  'validate-price-data',
  'register-contract',
  'unregister-contract',
  'set-status-funded',
];
const eventSourceAPIVersion = 'v0-1';

const chainType: Chain = 'STACKS';
const deployerPrincipal = config.admin_address;
const contractFullName = `${deployerPrincipal}.${contractName}`;
const dlcNFTName = `open-dlc`;
const registeredContractNFTName = `registered-contract`;
const eventSources = functionNames.map((name) => `dlclink:${name}:${eventSourceAPIVersion}`);

const handleTx = (tx: ContractCallTransaction) => {
  const unwrappedEvents = unwrapper(tx, eventSources, contractFullName);
  if (!unwrappedEvents.length) return;

  const currentTime = new Date().toLocaleString();

  unwrappedEvents.forEach((event) => {
    const { printEvent, eventSource } = event;
    if (!printEvent || !eventSource) return;

    switch (eventSource.event) {
      case 'create-dlc': {
        const _uuid = printEvent['uuid']?.value.value;
        const _emergencyRefundTime = printEvent['emergency-refund-time']?.value;
        const _creator = printEvent['creator']?.value;
        const _receiver = printEvent['receiver']?.value;
        const _callbackContract = printEvent['callback-contract']?.value;
        const _nonce = printEvent['nonce']?.value;
        const _logMessage = `[${contractName}] New DLC Request @ ${currentTime} \n\t uuid: ${_uuid} | emergencyRefund: ${timestampToDate(
          _emergencyRefundTime
        )} | caller: ${_creator} | callbackContract: ${_callbackContract} | callback_nonce: ${_nonce} \n`;
        // loggerSvc.log(_logMessage);

        try {
          (async () => {
            const response = await createRequestDirect({
              uuid: _uuid,
              emergencyRefundTime: _emergencyRefundTime,
              creator: _creator,
              receiver: _receiver,
              callbackContract: _callbackContract,
              nonce: _nonce,
              sourceContract: contractName,
              chain: chainType,
            });
            console.log('Response from create-direct: ', response);
          })();
        } catch (error) {
          console.error(error);
        }

        break;
      }

      case 'post-create-dlc': {
        const _uuid = printEvent['uuid']?.value;
        const _emergencyRefundTime = printEvent['emergency-refund-time']?.value;
        const _creator = printEvent['creator']?.value;
        const _logMessage = `[${contractName}] New DLC Created @ ${currentTime} \n\t uuid: ${_uuid} | emergencyRefund: ${timestampToDate(
          _emergencyRefundTime
        )} | caller: ${_creator} \n`;

        dlcInfoSvc.addInfo({
          uuid: _uuid,
          contractAddress: contractFullName,
          chain: chainType,
        });
        // loggerSvc.log(_logMessage);
        break;
      }

      case 'close-dlc': {
        const _uuid = printEvent['uuid']?.value;
        const _outcome = printEvent['outcome']?.value;
        const _callbackContract = printEvent['callback-contract']?.value;
        const _creator = printEvent['creator']?.value;
        const _logMessage = `[${contractName}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${_outcome} | creator: ${_creator} | callbackContract: ${_callbackContract}\n`;
        // loggerSvc.log(_logMessage);

        dlcInfoSvc.addInfo({
          uuid: _uuid,
          contractAddress: contractFullName,
          chain: chainType,
          outcome: _outcome,
        });

        (async () => {
          try {
            callPostCloseOutcome(
              {
                uuid: _uuid,
                callbackContract: _callbackContract,
                outcome: _outcome,
                assetInfo: {
                  contractAddress: config.admin_address,
                  contractName,
                  NFTName: dlcNFTName,
                },
              },
              2 // precisionShift
            );
          } catch (error) {
            console.error(error);
          }
        })();
        break;
      }

      case 'post-close-dlc': {
        const _uuid = printEvent['uuid']?.value;
        const _outcome = printEvent['outcome']?.value;
        const _actualClosingTime = printEvent['actual-closing-time']?.value;
        const _logMessage = `[${contractName}] Closed DLC @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${_outcome} | actualClosingTime: ${_actualClosingTime}\n`;
        // loggerSvc.log(_logMessage);
        break;
      }

      case 'get-btc-price': {
        const _uuid = printEvent['uuid']?.value;
        const _caller = printEvent['caller']?.value;
        const _creator = printEvent['creator']?.value;
        const _callbackContract = printEvent['callback-contract']?.value;
        const _logMessage = `[${contractName}] Price request... @ ${currentTime} \n\t uuid: ${_uuid} | caller: ${_caller} | callbackContract: ${_callbackContract}\n`;
        // loggerSvc.log(_logMessage);

        (async () => {
          callFetchPriceWithCallback({
            uuid: _uuid,
            callbackContract: _callbackContract,
            assetInfo: {
              contractAddress: config.admin_address,
              contractName,
              NFTName: dlcNFTName,
            },
            functionName: 'validate-price-data',
          });
        })();
        break;
      }

      case 'validate-price-data': {
        const _uuid = printEvent['uuid']?.value;
        const _price = printEvent['price']?.value;
        // loggerSvc.log(`[${contractName}] ${currentTime} Price Validation request for ${_uuid} with price ${_price}`);
        break;
      }

      case 'register-contract': {
        const _contractAddress = printEvent['contract-address']?.value;
        const _logMessage = `[${contractName}] ${currentTime} Contract registered on chain: ${_contractAddress}`;
        registrationSvc.registerContract(_contractAddress, contractConfig);
        // loggerSvc.log(_logMessage);
        break;
      }

      case 'unregister-contract': {
        const _contractAddress = printEvent['contract-address']?.value;
        const _logMessage = `[${contractName}] ${currentTime} Contract registration removed on chain: ${_contractAddress}`;
        registrationSvc.removeContractRegistration(_contractAddress);
        // loggerSvc.log(_logMessage);
        break;
      }

      case 'set-status-funded': {
        const _uuid = printEvent['uuid']?.value;
        const _callbackContract = printEvent['callback-contract']?.value;
        // loggerSvc.log(`[${contractName}] ${currentTime} Status set to funded for ${_uuid}`);
        break;
      }

      default: {
        // loggerSvc.log('Unknown event source');
        break;
      }
    }
  });
};

const getAddNewTxArguments = (params: AddStacksDirectDTO): ClarityValue[] => {
  const callbackContractPrincipal = parsePrincipalString(params.callbackContract) as ContractPrincipal;
  const creatorPrincipal = parsePrincipalString(params.creator);
  return [
    bufferCV(hexToBytes(params.uuid)),
    uintCV(parseInt(params.emergencyRefundTime || '0')),
    creatorPrincipal.prefix == 2
      ? standardPrincipalCV(addressToString(creatorPrincipal.address))
      : contractPrincipalCV(addressToString(creatorPrincipal.address), creatorPrincipal.contractName.content),
    contractPrincipalCV(
      addressToString(callbackContractPrincipal.address),
      callbackContractPrincipal.contractName.content
    ),
    uintCV(params.nonce),
  ];
};

export const contractConfig: ContractConfig = {
  chainType,
  deployerPrincipal,
  contractName,
  contractFullName,
  dlcNFTName,
  registeredContractNFTName,
  functionNames,
  handleTx,
  getAddNewTxArguments,
};
