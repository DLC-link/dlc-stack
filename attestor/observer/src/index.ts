import { Attestor } from 'attestor';
import { config } from './utilities/config';
import stacksInit from './chains/stacks/stacks.init';
import ethInit from './chains/ethereum/ethereum.init';

console.log('Starting attestor...');

export async function main() {
  const attestor = await Attestor.new();
  await attestor.create_event('event1', '2023-10-08T13:48:00Z');
  await attestor.attest('event1', 10n);

  const attestation = await attestor.get_event('event1');
  console.log('attested event1: ', attestation);

  console.table({
    'ETH Enabled': config.ethEnabled,
    'Stacks Enabled': config.stacksEnabled,
    'TX Handling Enabled': config.txHandlingEnabled,
    'Verbose Logs': config.verboseLogs,
  });
  // Initialize subscriptions:
  if (config.stacksEnabled) stacksInit();
  if (config.ethEnabled) ethInit();
}

main();
