import { ethers } from 'ethers';
import { DeploymentInfo } from '../../shared/models/deployment-info.interface.js';
import { BlockchainInterface } from '../../shared/models/observer.interface.js';
import AttestorService from '../../../services/attestor.service.js';
import { PrefixedChain, evmPrefix } from '../../../config/models.js';
import { createBlockchainObserverMetricsCounters } from '../../../config/prom-metrics.models.js';
import { TransactionReceipt } from '@ethersproject/abstract-provider';
import { EVMDLCInterface } from '../models/dlc-info.interface.js';

export const DlcManagerV1 = (contract: ethers.Contract, deploymentInfo: DeploymentInfo): BlockchainInterface => {
  const chainName = `${evmPrefix}${deploymentInfo.network.toLowerCase()}` as PrefixedChain;
  const ethereumObserverMetricsCounter = createBlockchainObserverMetricsCounters(chainName);

  function startListening() {
    contract.on(
      'CloseDLC',
      async (_uuid: string, _outcome: number, _protocolWallet: string, _sender: string, tx: any) => {
        ethereumObserverMetricsCounter.closeDLCEventCounter.inc();
        const currentTime = new Date();
        const outcome = BigInt(_outcome);
        const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${outcome} \n`;
        console.log(_logMessage);
        console.log('TXID:', tx.transactionHash);

        try {
          // NOTE: precision_shift is hardcoded to 2
          await AttestorService.createAttestation(_uuid, outcome, 2);
          console.log(await AttestorService.getEvent(_uuid));
        } catch (error) {
          console.error(error);
        }
      }
    );
  }

  async function getAllVaults(): Promise<any> {
    const vaults = await contract.getAllVaults();
    return vaults;
  }

  async function getDLCInfo(vaultUUID: string): Promise<EVMDLCInterface | undefined> {
    try {
      const dlcInfo = await contract.getDLC(vaultUUID);
      return dlcInfo;
    } catch (error) {
      console.log(error);
      throw error;
    }
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

  async function setVaultStatusFunded(vaultUUID: string, bitcoinTransactionID: string): Promise<TransactionReceipt> {
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
      throw error;
    }
  }

  async function setVaultStatusPostClosed(
    vaultUUID: string,
    bitcoinTransactionID: string
  ): Promise<TransactionReceipt> {
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
      throw error;
    }
  }

  return {
    chainName,
    startListening,
    getAllVaults,
    getDLCInfo,
    checkAndGetVault,
    setVaultStatusFunded,
    setVaultStatusPostClosed,
  };
};
