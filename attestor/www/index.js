import { Attestor } from "attestor";

async function go() {
  const attestor = await Attestor.new();

  const pubkey = await attestor.get_pubkey();
  console.log("pubkey: ", pubkey);

  await attestor.create_event("event1", "2023-10-08T13:48:00Z");
  await attestor.create_event("event2", "2023-10-08T13:48:00Z");
  await attestor.create_event("event3", "2023-10-08T13:48:00Z");

  let events = await attestor.get_events();
  console.log("events: ", events);

  console.log("Attesting event1");
  await attestor.attest("event1", 5n);

  console.log("Attesting event2");
  await attestor.attest("event2", 10n);

  console.log("Attesting event3");
  await attestor.attest("event3", 15n);

  const attestation = await attestor.get_event("event1");
  console.log("attested event1: ", attestation);

  const attestation2 = await attestor.get_event("event2");
  console.log("attested event2: ", attestation2);

  const attestation3 = await attestor.get_event("event3");
  console.log("attested event3: ", attestation3);

  events = await attestor.get_events();
  console.log("all events: ", events);
}

go();
