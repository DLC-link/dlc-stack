import AttestorService from './services/attestor.service.js';
import startServer from './http/server.js';
import setupPolyfills from './polyfills.js';
import ConfigService from './services/config.service.js';
import { getEthObserver } from './chains/ethereum/get-observer.js';
import getStacksObserver from './chains/stacks/get-observer.js';

async function main() {
  await AttestorService.init();

  // Set up necessary polyfills
  setupPolyfills();

  // Set up server with routes
  startServer();

  const evmChains = ConfigService.getEvmChainConfigs();
  const evmObservers = evmChains.map((config) => {
    return getEthObserver(config);
  });

  const stxChains = ConfigService.getStxChainConfigs();
  const stxObservers = stxChains.map((config) => getStacksObserver(config));

  const observers = await Promise.all([...evmObservers, ...stxObservers]);

  // Start observers
  observers.forEach((observer) => observer.start());
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
