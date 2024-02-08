import { ethers } from 'ethers';
import { DeploymentInfo } from '../models/ethereum-deployment-info.interface.js';
import { BlockchainInterface } from '../../../config/blockchain-interface.interface.js';
import AttestorService from '../../../services/attestor.service.js';
import { PrefixedChain, evmPrefix } from '../../../config/chains.models.js';
import { createBlockchainObserverMetricsCounters } from '../../../config/prom-metrics.models.js';
import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { EVMDLCInterface } from '../models/ethereum-dlc.interface.js';

export const DlcManagerV1 = (contract: ethers.Contract, deploymentInfo: DeploymentInfo): BlockchainInterface => {
  const chainName = `${evmPrefix}${deploymentInfo.network.toLowerCase()}` as PrefixedChain;
  const ethereumObserverMetricsCounter = createBlockchainObserverMetricsCounters(chainName);

  function startListening() {
    contract.on(
      'CloseDLC',
      async (_uuid: string, _outcome: number, _protocolWallet: string, _sender: string, tx: any) => {
        ethereumObserverMetricsCounter.closeDLCEventCounter.inc();
        const currentTime = new Date();
        const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid}\n`;
        console.log(_logMessage);

        try {
          const transactionID = await AttestorService.closePsbtEvent(_uuid);

          // TODO: create getPSBTEvent
          // console.log(await AttestorService.getEvent(_uuid));

          await setVaultStatusPostClosed(_uuid, transactionID);
        } catch (error) {
          console.error(error);
        }
      }
    );

    contract.on(
      'SetStatusFunded',
      async (_uuid: string, _btcTxId: string, _protocolWallet: string, _sender: string, tx: any) => {
        ethereumObserverMetricsCounter.setStatusFundedEventCounter.inc();
        const currentTime = new Date();
        const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] DLC funded @ ${currentTime} \n\t uuid: ${_uuid}\n`;
        console.log(_logMessage);

        try {
          await AttestorService.setPSBTEventToFunded(_uuid);
          // console.log(await AttestorService.getEvent(_uuid));
        } catch (error) {
          console.error(error);
        }
      }
    );

    // contract.on(
    //   'PostCloseDLC',
    //   async (_uuid: string, _outcome: number, _btcTxId: string, _protocolWallet: string, _sender: string, tx: any) => {
    //     ethereumObserverMetricsCounter.postCloseDLCEventCounter.inc();
    //     const currentTime = new Date();
    //     const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] DLC closed @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${_outcome} | btcTxId: ${_btcTxId} \n`;
    //     console.log(_logMessage);
    //     console.log('TXID:', tx.transactionHash);

    //     try {
    //       await AttestorService.set(_uuid);
    //       // console.log(await AttestorService.getEvent(_uuid));
    //     } catch (error) {
    //       console.error(error);
    //     }
    //   }
    // );
  }

  async function checkAndGetVault(vaultUUID: string): Promise<EVMDLCInterface | undefined> {
    try {
      const dlcInfo = await contract.getDLC(vaultUUID);
      if (dlcInfo?.uuid === vaultUUID) return dlcInfo;
    } catch (error) {
      console.log(error);
      throw new Error('Vault fetching failed');
    }
  }

  async function setVaultStatusFunded(vaultUUID: string, bitcoinTransactionID: string) {
    try {
      const gasLimit = await contract.estimateGas.setStatusFunded(vaultUUID, bitcoinTransactionID);
      const transaction = await contract.setStatusFunded(vaultUUID, bitcoinTransactionID, {
        gasLimit: gasLimit.add(10000),
      });
      const transactionReceipt = await transaction.wait();
      console.log('[SetStatusFunded] request transaction receipt: ', transactionReceipt);
      return transactionReceipt;
    } catch (error) {
      console.error(error);
      return error;
    }
  }

  async function setVaultStatusPostClosed(vaultUUID: string, bitcoinTransactionID: string) {
    try {
      const gasLimit = await contract.estimateGas.postCloseDLC(vaultUUID, bitcoinTransactionID);
      const transaction = await contract.postCloseDLC(vaultUUID, bitcoinTransactionID, {
        gasLimit: gasLimit.add(10000),
      });
      const transactionReceipt = await transaction.wait();
      console.log('[PostCloseDLC] request transaction receipt: ', transactionReceipt);
      return transactionReceipt;
    } catch (error) {
      console.log(error);
      return error;
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
