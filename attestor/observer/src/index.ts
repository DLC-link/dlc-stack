import { Attestor } from 'attestor';

async function main() {
  const attestor = await Attestor.new();
  await attestor.create_event('event1', '2023-10-08T13:48:00Z');
  await attestor.attest('event1', 10n);

  const attestation = await attestor.get_event('event1');
  console.log('attested event1: aslkdjalksdjalskjd ', attestation);
}

main();
