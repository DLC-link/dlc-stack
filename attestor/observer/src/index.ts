import AttestorService from './services/attestor.service.js';
import startServer from './http/server.js';
import setupPolyfills from './polyfills.js';
import ConfigService from './services/config.service.js';
import { getEthereumBlockchainInterface } from './chains/ethereum/get-ethereum-blockchain-interface.js';
// import getStacksObserver from './chains/stacks/get-observer.js';
import PeriodicService from './services/periodic.service.js';
import { getStackBlockchainInterface } from './chains/stacks/get-stacks-blockchain-interface.js';

async function main() {
  await AttestorService.init();

  // Set up necessary polyfills
  setupPolyfills();

  // Set up server with routes
  startServer();

  const evmChains = ConfigService.getEvmChainConfigs();
  const evmBlockchainInterfaces = evmChains.map((config) => {
    return getEthereumBlockchainInterface(config);
  });

  const stxChains = ConfigService.getStxChainConfigs();
  const stxBlockchainInterfaces = stxChains.map(async (config) => {
    return await getStackBlockchainInterface(config);
  });

  const blockchainInterfaces = await Promise.all([...evmBlockchainInterfaces, ...stxBlockchainInterfaces]);

  // Start observers
  blockchainInterfaces.forEach((blockchainInterface) => blockchainInterface.startListening());

  // Start periodic service
  PeriodicService.init(blockchainInterfaces);
  await PeriodicService.start(parseInt(process.env.PERIODIC_CHECK_FREQUENCY as string) || 10);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
