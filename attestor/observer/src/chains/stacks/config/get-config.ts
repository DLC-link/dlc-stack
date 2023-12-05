import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import { NetworkConfig } from '../models/interfaces.js';
import { io } from 'socket.io-client';
import { ChainConfig, stxPrefix } from '../../../config/models.js';
import ConfigService from '../../../services/config.service.js';

function setupSocketClient(endpoint: string): StacksApiSocketClient {
  const _socket = io(endpoint, {
    transports: ['websocket'],
    reconnection: true,
    reconnectionAttempts: Infinity,
    reconnectionDelay: 1000,
    reconnectionDelayMax: 5000,
    randomizationFactor: 0.5,
  });

  // NOTE: any
  const _stacksSocket: StacksApiSocketClient = new StacksApiSocketClient(_socket as any);

  _stacksSocket.socket.on('disconnect', async (reason: any) => {
    console.log(`[Stacks] Disconnected, reason: ${reason}`);
  });

  _stacksSocket.socket.on('connect', async () => {
    console.log('[Stacks] (Re)Connected stacksSocket');
  });

  setInterval(() => {
    if (_stacksSocket.socket.disconnected) {
      console.log(`[Stacks] Attempting to connect stacksSocket to ${endpoint}...`);
      _stacksSocket.socket.connect();
    }
  }, 2000);

  return _stacksSocket;
}

export function getConfig(config: ChainConfig): NetworkConfig {
  let socketEndpoint: string;
  let socket: StacksApiSocketClient;
  let deployer: string;
  let api_base_extended: string;

  switch (config.network) {
    case 'mainnet':
      deployer = '';
      api_base_extended = 'https://api.hiro.so/extended/v1';
      break;
    case 'testnet':
      deployer = 'ST1JHQ5GPQT249ZWG6V4AWETQW5DYA5RHJB0JSMQ3';
      api_base_extended = 'https://api.testnet.hiro.so/extended/v1';
      break;
    case 'mocknet':
      deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
      api_base_extended = `http://${ConfigService.getSettings()['mocknet-address']}:3999/extended/v1`;
      break;
    case 'local':
      deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
      api_base_extended = 'http://localhost:3999/extended/v1';
      break;
    default:
      throw new Error(`${config.network} is not a valid chain.`);
      break;
  }

  socketEndpoint = config.endpoint;
  socket = setupSocketClient(socketEndpoint);
  return { socket, deploymentInfo: { deployer, api_base_extended, chainName: `${stxPrefix}${config.network}` } };
}
