import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import { NetworkConfig } from '../models/interfaces.js';
import { io } from 'socket.io-client';
import { ChainConfig, stxPrefix } from '../../../config/models.js';

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

  if (!config.deployer) throw new Error('No deployer address provided');
  deployer = config.deployer;
  socketEndpoint = config.endpoint;
  api_base_extended = `${config.endpoint.replace('wss', 'https').replace('ws', 'http')}/extended/v1`;

  socket = setupSocketClient(socketEndpoint);
  return {
    socket,
    deploymentInfo: { deployer, api_base_extended, chainName: `${stxPrefix}${config.network}` },
  };
}
