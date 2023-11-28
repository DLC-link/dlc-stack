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

  // Load chain configs
  const chainConfigs = ConfigService.getChainConfigs();

  const observerPromises = chainConfigs.map((config) => {
    switch (config.type) {
      case 'EVM':
        return getEthObserver(config);
      case 'STX':
        return getStacksObserver(config);
      default:
        throw new Error(`${config.type} is not a valid type.`);
    }
  });

  const observers = await Promise.all(observerPromises);

  // Start observers
  observers.forEach((observer) => observer.start());
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
