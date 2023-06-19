import { Attestor } from "attestor";

async function go() {
  const attestor = await Attestor.new();
  await attestor.create_event("ashdlakshd", "2023-10-08T13:48:00Z");
}

go();
