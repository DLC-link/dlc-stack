import { getContracts } from './config/contract-list';
import DLCInfoService from '../../services/dlc-info.service';
import { config } from '../../utilities/config';
import { EthereumContract } from './models/classes/contract';

const dlcInfoSvc = DLCInfoService.getSvc();

export default () => {
  console.log('\n[Ethereum] Networks enabled:');
  const contracts = getContracts();
  console.table(config.ETHNetworks);
  dlcInfoSvc.ethLoadDLCData(contracts);
  listenForTXs(contracts);
};

function listenForTXs(contracts: EthereumContract[]) {
  contracts.forEach((contract) => {
    contract.handleTx();
    console.log(`[${contract.chain}][${contract.subchain}] ${contract.name} \t${contract.address}`);
    console.log(`[${contract.chain}][${contract.subchain}] BtcNft\t${contract.nftContractAddress}\n`);
  });
}

export async function checkStatus() {
  return `
  [ETHEREUM] DLCInfos: ${dlcInfoSvc.DLCInfo.filter((res) => res.chain == 'ETHEREUM').flatMap((elem) =>
    JSON.stringify({ uuid: elem.uuid })
  )}`;
}
