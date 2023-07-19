import { JsDLCInterface } from "dlc_protocol_wallet";

const testWalletPrivateKey =
  "f8ec31c12b6d014249935f2cb76b543b442ac2325993b44cbed4cdf773fbc8df";
const testWalletAddress = "bcrt1qatfjgacgqaua975r0cnsqtl09td8636jm3vnp0";

const bitcoinNetwork = "regtest";
const bitcoinNetworkURL = "https://dev-oracle.dlc.link/electrs";

const protocolWalletURL = "http://localhost:8085";

const handleAttestors = false;
const successfulAttesting = false;

// const testUUID = `test${Math.floor(Math.random() * 1000)}`;
const testUUID =
  "0x0ec6545ef52c10ab471c17432c828279b852e2d587e749aa9388af31ebc64ad8";

function createMaturationDate() {
  const maturationDate = new Date();
  maturationDate.setMinutes(maturationDate.getMinutes() + 1);
  return maturationDate.toISOString();
}

async function createEvent(attestorURL, uuid) {
  const maturationDate = createMaturationDate();
  const response = await fetch(
    `${attestorURL}/v1/create_event/${uuid}?maturation=${maturationDate}`
  );
  const event = await response.json();
  return event;
}

async function attest(attestorURL, uuid, outcome) {
  const response = await fetch(
    `${attestorURL}/v1/attest/${uuid}?outcome=${outcome}`
  );
  const event = await response.json();
  return event;
}

async function fetchOfferFromProtocolWallet() {
  let body = {
    uuid: testUUID,
    acceptCollateral: 10000,
    offerCollateral: 0,
    totalOutcomes: 100,
  };

  return fetch(`${protocolWalletURL}/offer`, {
    method: "post",
    body: JSON.stringify(body),
    headers: { "Content-Type": "application/json" },
  }).then((res) => res.json());
}

async function sendAcceptedOfferToProtocolWallet(accepted_offer) {
  return fetch(`${protocolWalletURL}/offer/accept`, {
    method: "put",
    body: JSON.stringify({
      acceptMessage: accepted_offer,
    }),
    headers: { "Content-Type": "application/json" },
  }).then((res) => res.json());
}

async function waitForBalance(dlcManager) {
  let balance = 0;
  while (balance <= 0) {
    balance = await dlcManager.get_wallet_balance();
    console.log("DLC Wasm Wallet Balance: " + balance);
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
}

async function runDLCFlow(dlcManager, dlcOffer) {
  console.log("Starting DLC flow");

  const acceptedContract = await dlcManager.accept_offer(
    JSON.stringify(dlcOffer)
  );
  console.log("Accepted Contract:", acceptedContract);

  const signedContract = await sendAcceptedOfferToProtocolWallet(
    acceptedContract
  );
  console.log("Signed Contract: ", signedContract);

  const txID = await dlcManager.countersign_and_broadcast(
    JSON.stringify(signedContract)
  );
  console.log(`Broadcast funding transaction with TX ID: ${txID}`);

  if (handleAttestors) {
    console.log("Attesting to Events");
    const attestations = await Promise.all(
      exampleAttestorURLs.map((attestorURL, index) =>
        attest(attestorURL, testUUID, successful ? 100 : index === 0 ? 0 : 100)
      )
    );
    console.log("Attestation received: ", attestations);
  }

  const contracts = await dlcManager.get_contracts();
  console.log("Contracts: ", contracts);
}

async function main() {
  console.log("DLC WASM Wallet Test");

  if (handleAttestors) {
    console.log("Creating Events");
    const events = await Promise.all(
      exampleAttestorURLs.map((attestorURL) =>
        createEvent(attestorURL, testUUID)
      )
    );
    console.log("Created Events: ", events);
  }

  console.log("Fetching Offer from Protocol Wallet");
  const offerResponse = await fetchOfferFromProtocolWallet();
  console.log("Received Offer (JSON): ", offerResponse[0]);
  console.log("Received Attestor URLs: ", offerResponse[1]);

  const joinedAttestorURLs = offerResponse[1]?.join(",");

  // creates a new instance of the JsDLCInterface
  const dlcManager = await JsDLCInterface.new(
    testWalletPrivateKey,
    testWalletAddress,
    bitcoinNetwork,
    bitcoinNetworkURL,
    joinedAttestorURLs
  );

  console.log("DLC Manager Interface Options: ", dlcManager.get_options());

  waitForBalance(dlcManager).then(() => {
    runDLCFlow(dlcManager, offerResponse[0]);
  });
}

main();
