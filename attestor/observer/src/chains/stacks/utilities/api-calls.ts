import { NFTHoldingsData } from '../models/interfaces.js';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';

export async function loadRegisteredContracts(
  api_base_extended: string,
  contractFullName: string,
  registeredContractNFTName: string
): Promise<NFTHoldingsData> {
  const registeredContractNFTsURL = `${api_base_extended}/tokens/nft/holdings?asset_identifiers=${contractFullName}::${registeredContractNFTName}&principal=${contractFullName}`;
  console.log(`[Stacks] Loading registered contracts from ${registeredContractNFTsURL}...`);
  try {
    const response = await fetch(registeredContractNFTsURL);
    return (await response.json()) as NFTHoldingsData;
  } catch (err) {
    console.error(err);
    throw err;
  }
}

export async function fetchTXInfo(txId: string, api_base_extended: string): Promise<ContractCallTransaction> {
  console.log(`[Stacks] Fetching tx_info... ${txId}`);
  try {
    const response = await fetch(api_base_extended + '/tx/' + txId);
    return (await response.json()) as ContractCallTransaction;
  } catch (err) {
    console.error(err);
    throw err;
  }
}
