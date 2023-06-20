import fetch from 'cross-fetch';
import { uuidResponseToString, uuidToCV } from '../utilities/helper-functions';
import { config as stacksConfig } from '../chains/stacks/config/networks.config';
import { ContractConfig } from '../chains/stacks/models/interfaces/contract-config.interface';
import { NFTHoldingsData } from '../chains/stacks/models/interfaces/nft-holdings.interface';
import { Chain } from '../chains/chain-types.interface';
import { EthereumContract } from '../chains/ethereum/models/classes/contract';
import { cvToHex, cvToValue, deserializeCV } from '@stacks/transactions';
import { BigNumber } from 'ethers';

export interface DLCInfo {
  uuid: string;
  contractAddress: string; // Stacks: contractConfig.contractFullName || ETH: Address
  chain: Chain;
  outcome?: number | null;
  fundedRequest?: boolean;
  funded?: boolean;
}

export default class DLCInfoService {
  private static instance: DLCInfoService;
  DLCInfo: Array<DLCInfo> = [];

  private constructor() {}

  public static getSvc(): DLCInfoService {
    if (!DLCInfoService.instance) DLCInfoService.instance = new DLCInfoService();
    return DLCInfoService.instance;
  }

  // Upserting info

  public addInfo(info: DLCInfo) {
    const i = this.DLCInfo.findIndex((_element) => _element.uuid === info.uuid);
    if (i > -1) this.DLCInfo[i] = info;
    else this.DLCInfo.push(info);
  }

  public removeInfo(uuid: string) {
    this.DLCInfo.splice(
      this.DLCInfo.findIndex((info) => info.uuid == uuid),
      1
    );
  }

  /* Loading all the UUIDs from the contract, and then fetching the DLC data for each one. */
  async ethLoadDLCData(contracts: Array<EthereumContract>) {
    contracts.forEach(async (contract) => {
      try {
        const uuids = await contract.loadUUIDs();
        uuids.forEach(async (res) => {
          const dlcdata = await contract.contractWithSigner.getDLC(res);
          const outcome = dlcdata.outcome as BigNumber;
          this.DLCInfo.push({
            uuid: res,
            contractAddress: contract.address,
            chain: contract.chain,
            outcome: outcome.toNumber(),
          });
        });
      } catch (error) {
        console.error(error);
        return;
      }
    });
  }

  async stacksLoadDLCData(contracts: Array<ContractConfig>) {
    contracts.forEach(async (contract) => {
      const openUUIDs = await this.stacksLoadOpenUUIDs(contract);
      openUUIDs.forEach(async (_uuid) => {
        const dlcData = await this.stacksFetchSingleDLCData(contract, _uuid);
        if (!dlcData) return;
        const _outcome = dlcData.value.outcome.value;
        this.DLCInfo.push({
          uuid: _uuid,
          contractAddress: contract.contractFullName,
          chain: contract.chainType,
          outcome: _outcome,
        });
      });
    });
  }

  async stacksLoadOpenUUIDs(contract: ContractConfig): Promise<string[]> {
    const openDLCsNFTsURL = `${stacksConfig.api_base_extended}/tokens/nft/holdings?asset_identifiers=${contract.contractFullName}::${contract.dlcNFTName}&principal=${contract.contractFullName}`;
    try {
      const resp = await fetch(openDLCsNFTsURL);
      const json = (await resp.json()) as any;
      const data: NFTHoldingsData = json;
      const results = data.results.map((res) => uuidResponseToString(res.value.repr));
      return results;
    } catch (error) {
      console.error(error);
      return [];
    }
  }

  async stacksFetchSingleDLCData(contract: ContractConfig, uuid: string) {
    try {
      const dlcreq = await fetch(
        `${stacksConfig.api_base}/v2/contracts/call-read/${contract.deployerPrincipal}/${contract.contractName}/get-dlc`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            sender: `${stacksConfig.admin_address}`,
            arguments: [cvToHex(uuidToCV(uuid))],
          }),
        }
      );
      const data = (await dlcreq.json()) as { okay: boolean; result: string };
      if (!data.okay) throw 'Failed to fetch DLC data';
      return cvToValue(deserializeCV(data.result));
    } catch (error) {
      console.error(error);
      return;
    }
  }
}
