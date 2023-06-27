import { ethers } from 'ethers';
import { DeploymentInfo } from '../../shared/models/deployment-info.interface.js';
import { Observer } from '../../shared/models/observer.interface.js';
import AttestorService from '../../../services/attestor.service.js';

export const DlcManagerV1 = (contract: ethers.Contract, deploymentInfo: DeploymentInfo): Observer => {
  return {
    start: () => {
      // bytes32 uuid,
      // string[] attestorList,
      // address creator,
      // address protocolWallet,
      // string eventSource

      contract.on(
        'CreateDLC',
        async (
          _uuid: string,
          _attestorList: string[],
          _creator: string,
          _protocolWallet: string,
          _eventSource: string
        ) => {
          const currentTime = new Date();
          const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] New DLC Request... @ ${currentTime} \n\t uuid: ${_uuid} | creator: ${_creator} | attestors: ${_attestorList} \n`;
          console.log(_logMessage);
          try {
            await AttestorService.createAnnouncement(_uuid);
            console.log(await AttestorService.getEvent(_uuid));
          } catch (error) {
            console.error(error);
          }
        }
      );

      // contract.on(
      //   'CloseDLC',
      //   async (_uuid: string, _outcome: ethers.BigNumber, _creator: string, _eventSource: string) => {
      //     const currentTime = new Date();
      //     const outcome = _outcome.toBigInt();
      //     const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${outcome} \n`;
      //     console.log(_logMessage);

      //     // TODO: precision_shift?
      //     try {
      //       await AttestorService.createAttestation(_uuid, outcome);
      //       console.log(await AttestorService.getEvent(_uuid));
      //     } catch (error) {
      //       console.error(error);
      //     }
      //   }
      // );
    },
  };
};
