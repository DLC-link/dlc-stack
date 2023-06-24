import { Attestor } from 'attestor';
import getObservers from './config/get-observers.js';
import { Observer } from './chains/shared/models/observer.interface.js';

function startServer() {}

function startObservers(observers: Observer[]) {
  observers.forEach((observer) => observer.start());
}

async function main() {
  // Set up server with routes
  startServer();

  // load observers
  const observers = await getObservers();
  console.log('observers: ', observers);

  // Start observers
  startObservers(observers);

  // TODO: attestor will move into its own service
  // const attestor = await Attestor.new();
  // await attestor.create_event('event1', '2023-10-08T13:48:00Z');
  // await attestor.attest('event1', 10n);

  // const attestation = await attestor.get_event('event1');
  // console.log('attested event1: aslkdjalksdjalskjd ', attestation);
}

main();
