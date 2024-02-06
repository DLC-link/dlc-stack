import AttestorService from './services/attestor.service.js';
import startServer from './http/server.js';
import setupPolyfills from './polyfills.js';
import ConfigService from './services/config.service.js';
import { getEthereumBlockchainInterface } from './chains/ethereum/get-observer.js';
// import getStacksObserver from './chains/stacks/get-observer.js';
import PeriodicService from './services/periodic.service.js';

async function main() {
  await AttestorService.init();

  // Set up necessary polyfills
  setupPolyfills();

  // Set up server with routes
  startServer();

  const evmChains = ConfigService.getEvmChainConfigs();
  const evmObservers = evmChains.map((config) => {
    return getEthereumBlockchainInterface(config);
  });

  const stxChains = ConfigService.getStxChainConfigs();
  // const stxObservers = stxChains.map((config) => getStacksObserver(config));

  const observers = await Promise.all([...evmObservers]);

  // Start observers
  observers.forEach((observer) => observer.startListening());

  // Start periodic service
  await PeriodicService.start(parseInt(process.env.PERIODIC_CHECK_FREQUENCY as string) || 10);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
