import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import { io as ioClient } from 'socket.io-client';
import fetch from 'cross-fetch';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';

import { config as stacksConfig } from './config/networks.config';
import { contracts as dlcManagerContracts } from './config/contract-list';
import ContractRegistrationService from './services/contract-registration.service';
import DLCInfoService from '../../services/dlc-info.service';
import { config } from '../../utilities/config';

let stacksSocket: StacksApiSocketClient;
let registrationSvc: ContractRegistrationService;
let dlcInfoSvc: DLCInfoService;

export default async () => {
  stacksSocket = initializeStacksSocket();
  registrationSvc = ContractRegistrationService.getSvc();
  registrationSvc.stacksSocket = stacksSocket; // important to initialize its socket!
  dlcInfoSvc = DLCInfoService.getSvc();

  console.log('\n[Stacks] Network config:');
  console.table({
    'Stacks Network': process.env.NETWORK,
    'API Base': stacksConfig.api_base,
    'Admin Address': stacksConfig.admin_address,
  });
  initializeStacksSubscriptions(stacksSocket);
  dlcInfoSvc.stacksLoadDLCData(dlcManagerContracts);
  listenForTXs();
};

function listenForTXs() {
  stacksSocket.socket.on('address-transaction', async (address, txWithTransfers) => {
    try {
      const tx = txWithTransfers.tx;
      if (tx.tx_status !== 'success') {
        console.log(`[Stacks] Skip - Failed tx: ${tx.tx_id}`);
        return;
      }

      if (tx.is_unanchored) {
        console.log(`[Stacks] Skip - Microblock tx: ${tx.tx_id}`);
        return;
      }

      // Incoming tx is either from a dlc-manager or a registered protocol contract:
      const contract =
        dlcManagerContracts.find((c) => c.contractFullName === address) ||
        registrationSvc._registeredProtocolContracts.find((reg) => reg.address === address);
      if (!contract) {
        console.log(`[Stacks] Skip - Heard event not belonging to a contract from address: ${address}`);
        return;
      }

      // We fetch detailed tx info
      const txInfo = await fetchTXInfo(tx.tx_id);
      if (txInfo.event_count < 1) {
        console.log(`[Stacks] Skip - Non-printing tx: ${tx.tx_id}`);
        return;
      }

      // Contracts themselves implement the handleTx function
      // TX_HANDLING_DISABLED is a flag that allows us to not run effectful oracle/wallet commands when testing
      if (config.txHandlingEnabled) {
        contract.handleTx(txInfo);
      }
    } catch (error) {
      console.error(error);
    }
  });
}

function initializeStacksSocket() {
  const _socket = ioClient(stacksConfig.ioclient_uri, {
    transports: ['websocket'],
  });

  const _stacksSocket = new StacksApiSocketClient(_socket);

  _stacksSocket.socket.on('disconnect', async (reason) => {
    console.log(`[Stacks] Disconnected, reason: ${reason}`);
  });

  _stacksSocket.socket.on('connect', async () => {
    console.log('[Stacks] (Re)Connected stacksSocket');
  });

  setInterval(() => {
    if (_stacksSocket.socket.disconnected) {
      console.log(`[Stacks] Attempting to connect stacksSocket to ${stacksConfig.ioclient_uri}...`);
      _stacksSocket.socket.connect();
    }
  }, 2000);

  return _stacksSocket;
}

function initializeStacksSubscriptions(_stacksSocket: StacksApiSocketClient) {
  dlcManagerContracts.forEach((contract) => {
    _stacksSocket.subscribeAddressTransactions(contract.contractFullName);
    console.log(`[Stacks] Listening to: ${contract.contractFullName}`);

    registrationSvc.loadRegisteredContracts(contract);
  });
}

async function fetchTXInfo(txId: string): Promise<ContractCallTransaction> {
  console.log(`[Stacks] Fetching tx_info... ${txId}`);
  try {
    const response = await fetch(stacksConfig.api_base_extended + '/tx/' + txId);
    return (await response.json()) as ContractCallTransaction;
  } catch (err) {
    console.error(err);
    throw err;
  }
}

export async function checkStatus() {
  console.log(stacksSocket.socket);
  return `
  Socket Connected: ${stacksSocket.socket.connected} \n \
  [STACKS] DLCInfos: ${dlcInfoSvc.DLCInfo.filter((res) => res.chain == 'STACKS').flatMap((elem) =>
    JSON.stringify({ uuid: elem.uuid })
  )}`;
}
