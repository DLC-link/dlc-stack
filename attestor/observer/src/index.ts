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
  const evmBIs = evmChains.map((config) => {
    return getEthereumBlockchainInterface(config);
  });

  const stxChains = ConfigService.getStxChainConfigs();
  // const stxBIs = stxChains.map((config) => getStacksObserver(config));

  const blockchainInterfaces = await Promise.all([...evmBIs]);

  // Start observers
  blockchainInterfaces.forEach((bi) => bi.startListening());

  // Start periodic service
  PeriodicService.init(blockchainInterfaces);
  await PeriodicService.start(parseInt(process.env.PERIODIC_CHECK_FREQUENCY as string) || 10);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
