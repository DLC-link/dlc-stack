import { AddressTransactionWithTransfers, StacksApiSocketClient } from '@stacks/blockchain-api-client';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import { Transaction } from '@stacks/stacks-blockchain-api-types';
import { BlockchainInterface } from '../../../config/blockchain-interface.interface.js';
import { PrefixedChain } from '../../../config/chains.models.js';
import { createBlockchainObserverMetricsCounters } from '../../../config/prom-metrics.models.js';
import AttestorService from '../../../services/attestor.service.js';
import { DeploymentInfo, FunctionName } from '../models/interfaces.js';
import unwrapper from '../utilities/unwrappers.js';
import { fetchTXInfo } from '../utilities/api-calls.js';
import {
  ContractPrincipal,
  SignedContractCallOptions,
  TxBroadcastResult,
  addressToString,
  broadcastTransaction,
  bufferCV,
  callReadOnlyFunction,
  contractPrincipalCV,
  cvToValue,
  makeContractCall,
  parsePrincipalString,
  stringAsciiCV,
} from '@stacks/transactions';
import { StacksNetwork } from '@stacks/network';
import { uuidToCV } from '../utilities/helper-functions.js';
import { StacksWallet } from '../models/stacks-wallet.model.js';
import StacksNonceService from '../../../services/stacks-nonce.service.js';
import { hexToBytes } from '@stacks/common';
import { BigNumber } from 'ethers';

const functionNames: Array<FunctionName> = [
  'create-dlc',
  'close-dlc',
  'post-close-dlc',
  'register-contract',
  'unregister-contract',
  'set-status-funded',
];

async function getCallbackContract(uuid: string, contractName: string, deployer: string, network: StacksNetwork) {
  const functionName = 'get-callback-contract';
  const txOptions = {
    contractAddress: deployer,
    contractName: contractName,
    functionName: functionName,
    functionArgs: [uuidToCV(uuid)],
    senderAddress: deployer,
    network: network,
  };
  const transaction: any = await callReadOnlyFunction(txOptions);
  const callbackContract = cvToValue(transaction.value);
  console.log(`Callback contract for uuid: '${uuid}':`, callbackContract);
  return parsePrincipalString(callbackContract) as ContractPrincipal;
}

export const DlcManagerV1 = async (
  socket: StacksApiSocketClient,
  deploymentInfo: DeploymentInfo,
  network: StacksNetwork,
  wallet: StacksWallet
): Promise<BlockchainInterface> => {
  const _socket = socket;
  const chainName = deploymentInfo.chainName as PrefixedChain;
  const contractName = 'dlc-manager-v1-1';
  const contractFullName = `${deploymentInfo.deployer}.dlc-manager-v1-1`;
  const stacksObserverMetricsCounter = createBlockchainObserverMetricsCounters(
    deploymentInfo.chainName as PrefixedChain
  );
  const eventSourceAPIVersion = 'v1';
  const eventSources = functionNames.map((name) => `dlclink:${name}:${eventSourceAPIVersion}`);

  _socket.subscribeAddressTransactions(contractFullName);

  function checkAddresses(address: string): boolean {
    return contractFullName == address;
  }

  async function handleTransaction(transaction: ContractCallTransaction) {
    console.log(`[Stacks] Received tx: ${transaction.tx_id}`);
    const unwrappedEvents = unwrapper(transaction, eventSources, contractName);
    if (!unwrappedEvents.length) return;
    unwrappedEvents.forEach(async (event) => {
      const { printEvent, eventSource } = event;
      if (!printEvent || !eventSource) return;
      const currentTime = new Date().toLocaleString();

      switch (eventSource.event) {
        case 'create-dlc': {
          console.log('create');
          break;
        }
        case 'close-dlc': {
          stacksObserverMetricsCounter.closeDLCEventCounter.inc();
          const _uuid = printEvent['uuid']?.value;
          const _logMessage = `[${contractName}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid}\n`;
          console.log(_logMessage);

          try {
            const transactionID = await AttestorService.closePsbtEvent(_uuid);
            await setVaultStatusPostClosed(_uuid, transactionID);
          } catch (error) {
            console.error(error);
          }
          break;
        }

        case 'set-status-funded': {
          stacksObserverMetricsCounter.setStatusFundedEventCounter.inc();
          const _uuid = printEvent['uuid']?.value;
          const _logMessage = `[${deploymentInfo.chainName}][${contractName}] DLC funded @ ${currentTime} \n\t uuid: ${_uuid}\n`;
          console.log(_logMessage);

          try {
            await AttestorService.setPSBTEventToFunded(_uuid);
            // console.log(await AttestorService.getEvent(_uuid));
          } catch (error) {
            console.error(error);
          }
          break;
        }
      }
    });
  }

  function startListening() {
    // console.log('socket:', _socket);

    _socket.socket.on(
      'address-transaction',
      async (address: string, txWithTransfers: AddressTransactionWithTransfers) => {
        console.log('heard smth');
        try {
          const tx = txWithTransfers.tx as Transaction;
          if (tx.tx_status !== 'success') {
            console.log(`[Stacks] Skip - Failed tx: ${tx.tx_id}`);
            return;
          }
          if (tx.is_unanchored) {
            console.log(`[Stacks] Skip - Microblock tx: ${tx.tx_id}`);
            return;
          }
          const txInfo = await fetchTXInfo(tx.tx_id, deploymentInfo.api_base_extended);
          if (txInfo.event_count < 1) {
            console.log(`[Stacks] Skip - Non-printing tx: ${tx.tx_id}`);
            return;
          }
          if (checkAddresses(address)) {
            await handleTransaction(txInfo);
          }
        } catch (error) {
          console.error(error);
        }
      }
    );
    _socket.socket.on('block', (block: any) => {
      console.log(`[Stacks] New block: ${block.height}`);
    });

    _socket.socket.on('transaction', (tx: any) => {
      console.log(`[Stacks] New tx: ${tx.tx_id}`);
    });

    // console.dir(_socket.socket, { depth: 5 });
  }

  async function checkAndGetVault(vaultUUID: string) {
    try {
      console.log('Getting DLC info...');
      const functionName = 'get-dlc';

      const transactionOptions = {
        contractAddress: deploymentInfo.deployer,
        contractName: contractName,
        functionName: functionName,
        functionArgs: [uuidToCV(vaultUUID)],
        senderAddress: deploymentInfo.deployer,
        network: network,
      };
      const transaction: any = await callReadOnlyFunction(transactionOptions);
      const dlcInfo = cvToValue(transaction.value);
      if (dlcInfo?.dlcUUID === vaultUUID) return dlcInfo;
    } catch (error) {
      console.log(error);
      return error;
    }
  }

  async function setVaultStatusFunded(vaultUUID: string, bitcoinTransactionID: string) {
    try {
      const callbackContractPrincipal = await getCallbackContract(
        vaultUUID,
        contractName,
        deploymentInfo.deployer,
        network
      );
      const functionName = 'set-status-funded';

      const transactionOptions: SignedContractCallOptions = {
        contractAddress: deploymentInfo.deployer,
        contractName: contractName,
        functionName: functionName,
        functionArgs: [
          uuidToCV(vaultUUID),
          stringAsciiCV(bitcoinTransactionID),
          contractPrincipalCV(
            addressToString(callbackContractPrincipal.address),
            callbackContractPrincipal.contractName.content
          ),
        ],
        senderKey: wallet.privateKey,
        validateWithAbi: true,
        network: network,
        anchorMode: 1,
        nonce: await StacksNonceService.getNonce(network, wallet.address),
      };

      const transaction = await makeContractCall(transactionOptions);
      console.log('Transaction payload:', transaction.payload);
      const broadcastResponse: TxBroadcastResult = await broadcastTransaction(transaction, network);
      console.log('Broadcast response: ', broadcastResponse);
      return broadcastResponse;
    } catch (error) {
      console.log(error);
      throw error;
    }
  }

  async function setVaultStatusPostClosed(vaultUUID: string, bitcoinTransactionID: string) {
    try {
      const callbackContractPrincipal = await getCallbackContract(
        vaultUUID,
        contractName,
        deploymentInfo.deployer,
        network
      );
      const functionName = 'post-close';

      const transactionOptions: SignedContractCallOptions = {
        contractAddress: deploymentInfo.deployer,
        contractName: contractName,
        functionName: functionName,
        functionArgs: [
          bufferCV(hexToBytes(vaultUUID)),
          stringAsciiCV(bitcoinTransactionID),
          contractPrincipalCV(
            addressToString(callbackContractPrincipal.address),
            callbackContractPrincipal.contractName.content
          ),
        ],
        senderKey: wallet.privateKey,
        validateWithAbi: true,
        network: network,
        anchorMode: 1,
        nonce: await StacksNonceService.getNonce(network, wallet.address),
      };

      const transaction = await makeContractCall(transactionOptions);
      console.log('Transaction payload:', transaction.payload);
      const broadcastResponse = await broadcastTransaction(transaction, network);
      console.log('Broadcast response: ', broadcastResponse);
      return broadcastResponse;
    } catch (error) {
      console.log(error);
      throw error;
    }
  }

  return {
    chainName,
    startListening,
    checkAndGetVault,
    setVaultStatusFunded,
    setVaultStatusPostClosed,
  };
};
