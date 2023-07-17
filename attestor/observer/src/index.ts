import AttestorService from './services/attestor.service.js';
import getObservers from './config/get-observers.js';
import { Observer } from './chains/shared/models/observer.interface.js';
import startServer from './http/server.js';
import setupPolyfills from './polyfills.js';

function startObservers(observers: Observer[]) {
  observers.forEach((observer) => observer.start());
}

async function testAttestorService() {
  await AttestorService.createAnnouncement('event1', '2023-10-08T13:48:00Z');
  await AttestorService.createAttestation('event1', 10n);
  const attestation = await AttestorService.getEvent('event1');
  console.log('attested event1:', attestation);
}

async function main() {
  // Set up necessary polyfills
  setupPolyfills();

  // Set up server with routes
  startServer();

  // Load observers
  const observers = await getObservers();

  // Start observers
  startObservers(observers);

  // Test attestor service
  // await testAttestorService();
}

main();
