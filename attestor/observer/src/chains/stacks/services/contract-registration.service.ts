import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import fetch from 'cross-fetch';
import { config as stacksConfig } from '../config/networks.config';
import { AddressSubscription } from '../models/interfaces/address-subscription.interface';
import { ContractConfig } from '../models/interfaces/contract-config.interface';
import { NFTHoldingsData } from '../models/interfaces/nft-holdings.interface';

export default class ContractRegistrationService {
  private static instance: ContractRegistrationService;
  _registeredProtocolContracts: Array<AddressSubscription> = [];
  _stacksSocket!: StacksApiSocketClient;

  public set stacksSocket(socket: StacksApiSocketClient) {
    this._stacksSocket = socket;
  }

  private constructor() {}

  public static getSvc(): ContractRegistrationService {
    if (!ContractRegistrationService.instance) {
      ContractRegistrationService.instance = new ContractRegistrationService();
    }
    return ContractRegistrationService.instance;
  }

  loadRegisteredContracts(dlcManager: ContractConfig) {
    const registeredContractNFTsURL = `${stacksConfig.api_base_extended}/tokens/nft/holdings?asset_identifiers=${dlcManager.contractFullName}::${dlcManager.registeredContractNFTName}&principal=${dlcManager.contractFullName}`;
    let data: NFTHoldingsData;
    fetch(registeredContractNFTsURL)
      .then((res) => res.json())
      .then((json) => (data = json as any))
      .catch((error) => console.error(error))
      .finally(() => {
        if (!data) return;
        data.results.map((res) => {
          const addy = res.value.repr.slice(1);
          // const addy = res.value.repr // Sometimes we need this. WTF?
          const leading = addy.slice(0, 1);
          if (leading == `'` || leading != 'S') {
            console.error(`Protocol contract format wrong: ${addy}`, -999);
            console.log('SLICE weirdness');
          }
          this._registeredProtocolContracts = this.registerContract(addy, dlcManager);
        });
      });
  }

  registerContract(_contractAddress: string, _dlcManager: ContractConfig) {
    const sub = this._registeredProtocolContracts.find((sub) => sub.address == _contractAddress);
    if (sub) {
      console.log(`[Stacks] Contract already registered.`);
      return this._registeredProtocolContracts;
    }
    console.log(`[Stacks] Registering protocol contract ${_contractAddress}`);
    this._registeredProtocolContracts.push({
      address: _contractAddress,
      subscription: this._stacksSocket.subscribeAddressTransactions(_contractAddress),
      handleTx: _dlcManager.handleTx,
    });
    return this._registeredProtocolContracts;
  }

  removeContractRegistration(_contractAddress: string) {
    const sub = this._registeredProtocolContracts.find((sub) => sub.address == _contractAddress);
    if (!sub) {
      console.log(`[Stacks] No such contract registered.`);
      return this._registeredProtocolContracts;
    }
    console.log(`[Stacks] Unregistering ${_contractAddress}`);
    sub.subscription.unsubscribe();
    this._registeredProtocolContracts.splice(this._registeredProtocolContracts.indexOf(sub), 1);
    return this._registeredProtocolContracts;
  }
}
