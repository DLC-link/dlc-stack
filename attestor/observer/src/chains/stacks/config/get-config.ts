import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import { NetworkConfig } from '../models/interfaces.js';
import { io } from 'socket.io-client';
import { ConfigSet } from '../../../config/models.js';
import { getEnv } from '../../../config/read-env-configs.js';

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

export function getConfig(config: ConfigSet): NetworkConfig {
  let socketEndpoint: string;
  let socket: StacksApiSocketClient;
  let deployer: string;
  let api_base_extended: string;

  switch (config.chain) {
    case 'STACKS_MAINNET':
      socketEndpoint = 'wss://api.hiro.so/';
      deployer = '';
      api_base_extended = 'https://api.hiro.so/extended/v1';
      break;
    case 'STACKS_TESTNET':
      socketEndpoint = 'wss://api.testnet.hiro.so/';
      deployer = 'ST1JHQ5GPQT249ZWG6V4AWETQW5DYA5RHJB0JSMQ3';
      api_base_extended = 'https://api.testnet.hiro.so/extended/v1';
      break;
    case 'STACKS_MOCKNET':
      socketEndpoint = `ws://${getEnv('MOCKNET_ADDRESS')}:3999/`;
      deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
      api_base_extended = `http://${getEnv('MOCKNET_ADDRESS')}:3999/extended/v1`;
      break;
    case 'STACKS_LOCAL':
      socketEndpoint = 'ws://localhost:3999/';
      deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
      api_base_extended = 'http://localhost:3999/extended/v1';
      break;
    default:
      throw new Error(`${config.chain} is not a valid chain.`);
      break;
  }

  socket = setupSocketClient(socketEndpoint);
  return { socket, deploymentInfo: { deployer, api_base_extended } };
}
