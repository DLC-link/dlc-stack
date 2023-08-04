import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import { ContractConfig, DeploymentInfo, AddressSubscription } from '../../models/interfaces.js';
import { loadRegisteredContracts } from '../../utilities/api-calls.js';

export class DlcManagerV1 implements ContractConfig {
  private _contractFullName: string;
  socket: StacksApiSocketClient;
  deploymentInfo: DeploymentInfo;
  registeredContractSubscriptions: Array<AddressSubscription> = [];

  constructor(socket: StacksApiSocketClient, deploymentInfo: DeploymentInfo) {
    this._contractFullName = `${deploymentInfo.deployer}.dlc-manager-v1`;
    this.socket = socket;
    this.deploymentInfo = deploymentInfo;
  }

  async init() {
    let registeredContracts = await loadRegisteredContracts(
      this.deploymentInfo.api_base_extended,
      this._contractFullName,
      'registered-contract'
    );

    registeredContracts.results.forEach((result) => {
      this.registeredContractSubscriptions.push({
        address: result.value.repr.slice(1),
        subscription: this.socket.subscribeAddressTransactions(result.value.repr.slice(1)),
        handleTx: this.handleTx,
      });
    });

    this.socket.subscribeAddressTransactions(this._contractFullName);
    console.log(`[Stacks] Subscribed to ${this._contractFullName}`);
    console.log(`[Stacks] Loaded registered contracts:`, this.registeredContractSubscriptions);
  }

  checkAddresses(address: string): boolean {
    return (
      this._contractFullName == address ||
      this.registeredContractSubscriptions.some((subscription) => subscription.address === address)
    );
  }

  handleTx(tx: ContractCallTransaction) {
    console.log(`[Stacks] Received tx: ${tx.tx_id}`);
  }
}
