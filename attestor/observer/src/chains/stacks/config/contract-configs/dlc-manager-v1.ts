import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import { ContractConfig, DeploymentInfo, AddressSubscription, FunctionName } from '../../models/interfaces.js';
import { loadRegisteredContracts } from '../../utilities/api-calls.js';
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
  registeredContractSubscriptions: Array<AddressSubscription> = [];

  constructor(socket: StacksApiSocketClient, deploymentInfo: DeploymentInfo) {
    this._contractFullName = `${deploymentInfo.deployer}.dlc-manager-v1`;
    this._socket = socket;
    this._deploymentInfo = deploymentInfo;
  }

  async init() {
    let registeredContracts = await loadRegisteredContracts(
      this._deploymentInfo.api_base_extended,
      this._contractFullName,
      'registered-contract'
    );

    registeredContracts.results.forEach((result) => {
      this.registeredContractSubscriptions.push({
        address: result.value.repr.slice(1),
        subscription: this._socket.subscribeAddressTransactions(result.value.repr.slice(1)),
        handleTx: this.handleTx,
      });
    });

    this._socket.subscribeAddressTransactions(this._contractFullName);
    console.log(`[Stacks] Subscribed to ${this._contractFullName}`);
    console.log(`[Stacks] Loaded registered contracts:`, this.registeredContractSubscriptions);
  }

  checkAddresses(address: string): boolean {
    return (
      this._contractFullName == address ||
      this.registeredContractSubscriptions.some((subscription) => subscription.address === address)
    );
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
          console.dir(printEvent, { depth: 5 });
          const _uuid = printEvent['uuid']?.value;
          const _creator = printEvent['creator']?.value;
          const _callbackContract = printEvent['callback-contract']?.value;
          const _protocolWallet = printEvent['protocol-wallet']?.value;
          const _attestors = printEvent['attestors']?.value.flatMap((res: any) => res.value.dns.value);
          const _logMessage = `[${this._contractFullName}] New DLC Request @ ${currentTime} \n\t uuid: ${_uuid} | creator: ${_creator} | callbackContract: ${_callbackContract} | protocol-wallet: ${_protocolWallet} | attestors: ${_attestors} \n`;
          console.log(_logMessage);
          try {
            await AttestorService.createAnnouncement(_uuid);
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
          this.registeredContractSubscriptions.push({
            address: _contractAddress,
            subscription: this._socket.subscribeAddressTransactions(_contractAddress),
            handleTx: this.handleTx,
          });
          console.log(_logMessage);
          break;
        }

        case 'unregister-contract': {
          const _contractAddress = printEvent['contract-address']?.value;
          const _logMessage = `[${this._contractFullName}] ${currentTime} Contract registration removed on chain: ${_contractAddress}`;
          const sub = this.registeredContractSubscriptions.find((sub) => sub.address === _contractAddress);
          if (sub) {
            sub.subscription.unsubscribe();
            this.registeredContractSubscriptions = this.registeredContractSubscriptions.filter(
              (sub) => sub.address !== _contractAddress
            );
          }
          console.log(_logMessage);
          break;
        }
      }
    });
  }
}
