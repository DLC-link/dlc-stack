import dotenv from "dotenv";
dotenv.config();
import { JsDLCInterface } from "./node_modules/wasm-wallet/dlc_wasm_wallet.js";
import fetch from "cross-fetch";
import config from "./config.js";
import setupPolyfills from "./polyfills.js";

setupPolyfills();

const {
  testWalletPrivateKey,
  testWalletAddress,
  bitcoinNetwork,
  bitcoinNetworkURL,
  protocolWalletURL,
  attestorList,
} = config;

const handleAttestors = process.env.HANDLE_ATTESTORS == "true";
const testUUID = process.env.UUID || `test${Math.floor(Math.random() * 1000)}`;
const successfulAttesting = process.env.SUCCESSFUL_ATTESTING == "true";

// const attestorListReplaced = attestorList.map((attestorURL) =>
//   attestorURL.replace("localhost", "host.docker.internal")
// );

function createMaturationDate() {
  const maturationDate = new Date();
  maturationDate.setMinutes(maturationDate.getMinutes() + 1);
  return maturationDate.toISOString();
}

async function createEvent(attestorURL, uuid) {
  const maturationDate = createMaturationDate();
  try {
    const url = `${attestorURL}/create-announcement/${uuid}`;
    console.log("Creating event at: ", url);
    const response = await fetch(url);
    const event = await response.json();
    return event;
  } catch (error) {
    console.error("Error creating event: ", error);
    process.exit(1);
  }
}

async function attest(attestorURL, uuid, outcome) {
  try {
    const response = await fetch(
      `${attestorURL}/create-attestation/${uuid}/${outcome}`
    );
    const event = await response.json();
    return event;
  } catch (error) {
    console.error("Error attesting: ", error);
    process.exit(1);
  }
}

async function fetchOfferFromProtocolWallet() {
  let body = {
    uuid: testUUID,
    acceptCollateral: 10000,
    offerCollateral: 0,
    totalOutcomes: 100,
    attestorList: JSON.stringify(attestorList),
  };

  try {
    const res = await fetch(`${protocolWalletURL}/offer`, {
      method: "post",
      body: JSON.stringify(body),
      headers: { "Content-Type": "application/json" },
    });
    return await res.json();
  } catch (error) {
    console.error("Error fetching offer: ", error);
    process.exit(1);
  }
}

async function sendAcceptedOfferToProtocolWallet(accepted_offer) {
  try {
    const res = await fetch(`${protocolWalletURL}/offer/accept`, {
      method: "put",
      body: JSON.stringify({
        acceptMessage: accepted_offer,
      }),
      headers: { "Content-Type": "application/json" },
    });
    return await res.json();
  } catch (error) {
    console.error("Error sending accepted offer: ", error);
    process.exit(1);
  }
}

async function unlockUTXOsInProtocolWallet() {
  try {
    const res = await fetch(`${protocolWalletURL}/unlockutxos`, {
      method: "put",
      headers: { "Content-Type": "application/json" },
    });
    return await res.json();
  } catch (error) {
    console.error("Error unlocking UTXOs: ", error);
    process.exit(1);
  }
}

async function fetchTxDetails(txId) {
  try {
    const res = await fetch(`https://devnet.dlc.link/electrs/tx/${txId}`, {
      method: "get",
      headers: { "Content-Type": "application/json" },
    });
    return await res.json();
  } catch (error) {
    console.error("Error fetching funding tx, maybe the broadcast failed?: ", error);
    process.exit(1);
  }
}

async function waitForBalance(dlcManager) {
  let balance = 0;
  while (balance <= 0) {
    balance = await dlcManager.get_wallet_balance();
    console.log("DLC Wasm Wallet Balance: " + balance);
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
}

async function main() {
  console.log("DLC Integration Test flow");

  // TODO:
  // - wait for protocol wallet to be ready
  // - check& retry for protocol wallet balance

  if (handleAttestors) {
    console.log("Creating Events");
    const events = await Promise.all(
      attestorList.map((attestorURL) => createEvent(attestorURL, testUUID))
    );
    console.log("Created Events: ", events);
  }

  // console.log("Unlocking UTXOs in Protocol Wallet");
  // await unlockUTXOsInProtocolWallet();

  console.log("Fetching Offer from Protocol Wallet");
  const offerResponse = await fetchOfferFromProtocolWallet();
  if (!offerResponse.temporaryContractId) {
    console.log("Error fetching offer from protocol wallet: ", offerResponse);
    process.exit(1);
  }
  console.log("Received Offer (JSON): ", offerResponse);

  // creates a new instance of the JsDLCInterface
  const dlcManager = await JsDLCInterface.new(
    testWalletPrivateKey,
    testWalletAddress,
    bitcoinNetwork,
    bitcoinNetworkURL,
    JSON.stringify(attestorList)
  );

  console.log("DLC Manager Interface Options: ", dlcManager.get_options());

  await waitForBalance(dlcManager);

  const acceptedContract = await dlcManager.accept_offer(
    JSON.stringify(offerResponse)
  );
  const pared_response = JSON.parse(acceptedContract);
  if (!pared_response.protocolVersion) {
    console.log("Error accepting offer: ", pared_response);
    process.exit(1);
  }
  console.log("Contract accepted: ", pared_response);

  const signedContract = await sendAcceptedOfferToProtocolWallet(
    acceptedContract
  );
  console.log("Contract signed: ", signedContract);

  const txID = await dlcManager.countersign_and_broadcast(
    JSON.stringify(signedContract)
  );
  console.log(`Broadcast funding transaction with TX ID: ${txID}`);

  // wait for 3 seconds for electrs to get it's act together
  console.log("Fetching tx details to make sure the broadcast was successful...");
  await new Promise((resolve) => setTimeout(resolve, 3000));
  // fetch the tx details from electrs to make sure it was broadcasted
  const txDetails = await fetchTxDetails(txID);
  console.log("Funding TX Details: {}", txDetails);

  if (handleAttestors) {
    console.log("Attesting to Events");
    const attestations = await Promise.all(
      attestorList.map((attestorURL, index) =>
        attest(
          attestorURL,
          testUUID,
          successfulAttesting ? 100 : index === 0 ? 0 : 100
        )
      )
    );
    console.log("Attestation received: ", attestations);
  }

  const contracts = await dlcManager.get_contracts();
  console.log("Contracts: ", contracts);
}

main();
