import { StacksApiSocketClient } from '@stacks/blockchain-api-client';
import { connectWebSocketClient } from '@stacks/blockchain-api-client';
import { io } from 'socket.io-client';
import { Chain, ChainConfig, stxPrefix } from '../../config/chains.models.js';
import { NetworkConfig } from './models/interfaces.js';

let networkConfigMap: Map<Chain, NetworkConfig> = new Map();

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

  // TODO: FIXME: It does not hear events actually.
  _stacksSocket.subscribeBlocks();

  _stacksSocket.subscribeAddressTransactions('ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM');

  setInterval(() => {
    if (_stacksSocket.socket.disconnected) {
      console.log(`[Stacks] Attempting to connect stacksSocket to ${endpoint}...`);
      _stacksSocket.socket.connect();
    }
  }, 2000);

  return _stacksSocket;
}

export default async (config: ChainConfig): Promise<NetworkConfig> => {
  // we don't want to create a new network config for every request
  const existingNetworkConfig = networkConfigMap.get(config.network);
  if (existingNetworkConfig) {
    return existingNetworkConfig;
  }

  let socketEndpoint: string;
  let socket: StacksApiSocketClient;
  const { deployer, endpoint } = config;
  if (!deployer) throw new Error(`[Stacks] No deployer address found in config.`);
  const api_base_extended = `${endpoint}/extended/v1`;

  if (!config.deployer) throw new Error('No deployer address provided');

  socketEndpoint = endpoint;
  //   api_base_extended = `${config.endpoint.replace('wss', 'https').replace('ws', 'http')}/extended/v1`;

  socket = setupSocketClient(socketEndpoint);

  const deploymentInfo = { deployer, api_base_extended, chainName: `${stxPrefix}${config.network}` };
  const networkConfig = { socket, deploymentInfo };
  networkConfigMap.set(config.network, networkConfig);
  return networkConfig;
};
