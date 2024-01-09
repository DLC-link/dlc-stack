import { ethers } from 'ethers';
import { DeploymentInfo } from '../../shared/models/deployment-info.interface.js';
import { Observer } from '../../shared/models/observer.interface.js';
import AttestorService from '../../../services/attestor.service.js';
import { PrefixedChain, evmPrefix } from '../../../config/models.js';
import { createBlockchainObserverMetricsCounters } from '../../../config/prom-metrics.models.js';

export const DlcManagerV1 = (contract: ethers.Contract, deploymentInfo: DeploymentInfo): Observer => {
  const chainName = `${evmPrefix}${deploymentInfo.network.toLowerCase()}` as PrefixedChain;
  const ethereumObserverMetricsCounter = createBlockchainObserverMetricsCounters(chainName);

  return {
    start: () => {
      contract.on(
        'CreateDLC',
        async (
          _uuid: string,
          _valueLocked: string,
          _protocolContract: string,
          _creator: string,
          _protocolWallet: string,
          _timestamp: string,
          tx: any
        ) => {
          ethereumObserverMetricsCounter.createDLCEventCounter.inc();
          const currentTime = new Date();
          const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] New DLC Request... @ ${currentTime} \n\t uuid: ${_uuid} | creator: ${_creator} | timestamp: ${_timestamp} \n`;
          console.log(_logMessage);
          console.log('TXID:', tx.transactionHash);
          try {
            await AttestorService.createAnnouncement(_uuid, chainName);
            console.log(await AttestorService.getEvent(_uuid));
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
          const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] DLC funded @ ${currentTime} \n\t uuid: ${_uuid} | protocolWallet: ${_protocolWallet} | sender: ${_sender} \n`;
          console.log(_logMessage);
          console.log('TXID:', tx.transactionHash);
        }
      );

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

      contract.on(
        'PostCloseDLC',
        async (
          _uuid: string,
          _outcome: number,
          _btcTxId: string,
          _protocolWallet: string,
          _sender: string,
          tx: any
        ) => {
          ethereumObserverMetricsCounter.postCloseDLCEventCounter.inc();
          const currentTime = new Date();
          const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] DLC closed @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${_outcome} | btcTxId: ${_btcTxId} \n`;
          console.log(_logMessage);
          console.log('TXID:', tx.transactionHash);
        }
      );
    },
  };
};
