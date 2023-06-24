import { ethers } from 'ethers';
import { DeploymentInfo } from '../../shared/models/deployment-info.interface.js';
import { Observer } from '../../shared/models/observer.interface.js';
import AttestorService from '../../../services/attestor.service.js';

export const DlcManagerV0 = (contract: ethers.Contract, deploymentInfo: DeploymentInfo): Observer => {
  return {
    start: () => {
      contract.on(
        'CreateDLC',
        async (
          _uuid: string,
          _creator: string,
          _receiver: string,
          _emergencyRefundTime: ethers.BigNumber,
          _nonce: ethers.BigNumber,
          _eventSource: string
        ) => {
          const currentTime = new Date();
          const emergencyRefundTime = _emergencyRefundTime.toNumber().toString();
          const nonce = _nonce.toNumber().toString() as any;
          const _logMessage = `[${deploymentInfo.network}][${deploymentInfo.contract.name}] New DLC Request... @ ${currentTime} \n\t uuid: ${_uuid} | emergencyRefundTime: ${emergencyRefundTime} | creator: ${_creator} \n`;
          console.log(_logMessage);
          // loggerSvc.log(_logMessage);
          try {
            await AttestorService.createAnnouncement(_uuid);
          } catch (error) {
            console.error(error);
          }
        }
      );
    },
  };
};
