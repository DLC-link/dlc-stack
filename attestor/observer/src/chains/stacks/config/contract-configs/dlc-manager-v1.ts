import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import { ContractConfig, DeploymentInfo, FunctionName } from '../../models/interfaces.js';
import unwrapper from '../../utilities/unwrappers.js';
import AttestorService from '../../../../services/attestor.service.js';

export class DlcManagerV1 implements ContractConfig {
  private _contractFullName: string;
  private _socket: StacksApiSocketClient;
  private _deploymentInfo: DeploymentInfo;
  private _functionNames: Array<FunctionName> = [
    'create-dlc',
    'close-dlc',
    'post-close-dlc',
    'register-contract',
    'unregister-contract',
    'set-status-funded',
  ];
  private _eventSourceAPIVersion = 'v1';
  private _eventSources = this._functionNames.map((name) => `dlclink:${name}:${this._eventSourceAPIVersion}`);

  constructor(socket: StacksApiSocketClient, deploymentInfo: DeploymentInfo) {
    this._contractFullName = `${deploymentInfo.deployer}.dlc-manager-v1`;
    this._socket = socket;
    this._deploymentInfo = deploymentInfo;
  }

  async init() {
    this._socket.subscribeAddressTransactions(this._contractFullName);
    console.log(`[Stacks] Subscribed to ${this._contractFullName}`);
  }

  checkAddresses(address: string): boolean {
    return this._contractFullName == address;
  }

  async handleTx(tx: ContractCallTransaction) {
    console.log(`[Stacks] Received tx: ${tx.tx_id}`);
    const unwrappedEvents = unwrapper(tx, this._eventSources, this._contractFullName);
    if (!unwrappedEvents.length) return;

    unwrappedEvents.forEach(async (event) => {
      const { printEvent, eventSource } = event;
      if (!printEvent || !eventSource) return;
      const currentTime = new Date().toLocaleString();

      switch (eventSource.event) {
        case 'create-dlc': {
          const _uuid = printEvent['uuid']?.value;
          const _creator = printEvent['creator']?.value;
          const _callbackContract = printEvent['callback-contract']?.value;
          const _protocolWallet = printEvent['protocol-wallet']?.value;
          const _refundDelay = printEvent['refund-delay']?.value;
          const _valueLocked = printEvent['value-locked']?.value;
          const _logMessage = `[${this._contractFullName}] New DLC Request @ ${currentTime} \n\t uuid: ${_uuid} | creator: ${_creator} | callbackContract: ${_callbackContract} | protocol-wallet: ${_protocolWallet} | value-locked: ${_valueLocked} | refund-delay: ${_refundDelay} \n`;
          console.log(_logMessage);
          try {
            await AttestorService.createAnnouncement(_uuid, this._deploymentInfo.chainName);
            console.log(await AttestorService.getEvent(_uuid));
          } catch (error) {
            console.error(error);
          }
          break;
        }

        case 'close-dlc': {
          const _uuid = printEvent['uuid']?.value;
          const _outcome = printEvent['outcome']?.value;
          const _creator = printEvent['creator']?.value;
          const _logMessage = `[${this._contractFullName}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${_outcome} | creator: ${_creator}\n`;
          console.log(_logMessage);
          try {
            await AttestorService.createAttestation(_uuid, _outcome);
            console.log(await AttestorService.getEvent(_uuid));
          } catch (error) {
            console.error(error);
          }
          break;
        }

        case 'post-close-dlc': {
          console.log(`[Stacks] Received post-close-dlc event`);
          break;
        }

        case 'set-status-funded': {
          const _uuid = printEvent['uuid']?.value;
          console.log(`[${this._contractFullName}] ${currentTime} Status set to funded for ${_uuid}`);
          break;
        }

        case 'register-contract': {
          const _contractAddress = printEvent['contract-address']?.value;
          const _logMessage = `[${this._contractFullName}] ${currentTime} Contract registered on chain: ${_contractAddress}`;
          console.log(_logMessage);
          break;
        }

        case 'unregister-contract': {
          const _contractAddress = printEvent['contract-address']?.value;
          const _logMessage = `[${this._contractFullName}] ${currentTime} Contract registration removed on chain: ${_contractAddress}`;
          console.log(_logMessage);
          break;
        }
      }
    });
  }
}
