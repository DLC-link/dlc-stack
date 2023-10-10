import dotenv from 'dotenv';
dotenv.config();
import { JsDLCInterface } from '../node_modules/wasm-wallet/dlc_tools.js';
import fetch from 'cross-fetch';
import config from './config.js';
import setupPolyfills from './polyfills.js';

setupPolyfills();

const { testWalletPrivateKey, testWalletAddress, bitcoinNetwork, bitcoinNetworkURL, protocolWalletURL, attestorList } =
  config;

const handleAttestors = process.env.HANDLE_ATTESTORS == 'true';
const testUUID = process.env.UUID || `test${Math.floor(Math.random() * 1000)}`;
const successfulAttesting = process.env.SUCCESSFUL_ATTESTING == 'true';

const acceptorGetsAllOutcome = 0;
const offererGetsAllOutcome = 100;

const totalOutcomes = 100;

const acceptCollateral = 10000;

async function createEvent(attestorURL, uuid) {
  try {
    const url = `${attestorURL}/create-announcement/${uuid}`;
    const response = await fetch(url);
    const event = await response.json();
    return event;
  } catch (error) {
    console.error('Error creating event: ', error);
    process.exit(1);
  }
}

async function attest(attestorURL, uuid, outcome) {
  try {
    const response = await fetch(`${attestorURL}/create-attestation/${uuid}/${outcome}`);
    const event = await response.json();
    return event;
  } catch (error) {
    console.error('Error attesting: ', error);
    process.exit(1);
  }
}

async function fetchOfferFromProtocolWallet() {
  let body = {
    uuid: testUUID,
    acceptCollateral: acceptCollateral,
    offerCollateral: 0,
    totalOutcomes: totalOutcomes,
    attestorList: JSON.stringify(attestorList),
  };

  try {
    const res = await fetch(`${protocolWalletURL}/offer`, {
      method: 'post',
      body: JSON.stringify(body),
      headers: { 'Content-Type': 'application/json' },
    });
    return await res.json();
  } catch (error) {
    console.error('Error fetching offer: ', error);
    process.exit(1);
  }
}

async function sendAcceptedOfferToProtocolWallet(accepted_offer) {
  try {
    const res = await fetch(`${protocolWalletURL}/offer/accept`, {
      method: 'put',
      body: JSON.stringify({
        acceptMessage: accepted_offer,
      }),
      headers: { 'Content-Type': 'application/json' },
    });
    return await res.json();
  } catch (error) {
    console.error('Error sending accepted offer: ', error);
    process.exit(1);
  }
}

async function checkBalance(dlcManager, action) {
  const balance = await dlcManager.get_wallet_balance();
  console.log(`DLC Wasm Wallet Balance at ${action}: ` + balance);
  return balance;
}

async function fetchTxDetails(txId) {
  const url = `${process.env.ELECTRUM_API_URL}/tx/${txId}`;
  try {
    const res = await fetch(url, {
      method: 'get',
      headers: { 'Content-Type': 'application/json' },
    });
    return await res.json();
  } catch (error) {
    console.error('Error fetching Funding TX, the broadcast possibly failed', error);
    process.exit(1);
  }
}

async function retry(checkFunction, timeoutTime) {
  let timeRemaining = timeoutTime;
  while (timeRemaining) {
    const result = await checkFunction();
    if (result) return true;
    await new Promise((resolve) => setTimeout(resolve, 2000));
    timeRemaining -= 2000;
  }
  return false;
}

function assert(predicate, message) {
  if (!predicate) {
    console.error(message);
    process.exit(1);
  }
}

async function waitForConfirmations(blockchainHeightAtBroadcast, targetConfirmations) {
  const url = `${process.env.ELECTRUM_API_URL}/blocks/tip/height`;
  let currentBlockchainHeight = blockchainHeightAtBroadcast;
  while (Number(currentBlockchainHeight) - Number(blockchainHeightAtBroadcast) < targetConfirmations) {
    currentBlockchainHeight = await (await fetch(url)).json();
    console.log(
      `Confirmations: ${Number(currentBlockchainHeight) - Number(blockchainHeightAtBroadcast)} / ${targetConfirmations}`
    );
    await new Promise((resolve) => setTimeout(resolve, 15000));
  }
  return true;
}

async function checkIfContractIsInState(contractID, state) {
  const routerWalletInfo = await (await fetch(`${protocolWalletURL}/info`)).json();
  return routerWalletInfo.contracts[state].includes(contractID);
}

async function getBlockchainHeight() {
  const url = `${process.env.ELECTRUM_API_URL}/blocks/tip/height`;
  const currentBlockchainHeight = await (await fetch(url)).json();
  return currentBlockchainHeight;
}

async function main() {
  console.log('Starting DLC Integration Test');

  //Creating Events
  if (handleAttestors) {
    console.log('Creating Events');
    const events = await Promise.all(attestorList.map((attestorURL) => createEvent(attestorURL, testUUID)));
    console.log('Created Events: ', events);
  }

  //Fetching Offer
  console.log('Fetching Offer from Protocol Wallet');
  const offerResponse = await fetchOfferFromProtocolWallet();

  //Check if the offer is valid
  if (!offerResponse.temporaryContractId) {
    console.error('Error fetching offer from protocol wallet: ', offerResponse);
    process.exit(1);
  }

  //Creating DLC Manager Interface
  const dlcManager = await JsDLCInterface.new(
    testWalletPrivateKey,
    testWalletAddress,
    bitcoinNetwork,
    bitcoinNetworkURL,
    JSON.stringify(attestorList)
  );

  //Checking Balance
  const startingBalance = await checkBalance(dlcManager, '[TEST START]');

  //Accepting Offer
  const acceptedContract = await dlcManager.accept_offer(JSON.stringify(offerResponse));
  const parsedResponse = JSON.parse(acceptedContract);

  //Check if the accepted contract is valid
  if (!parsedResponse.protocolVersion) {
    console.log('Error accepting offer: ', parsedResponse);
    process.exit(1);
  }

  //Sending Accepted Offer to Protocol Wallet
  const signedContract = await sendAcceptedOfferToProtocolWallet(acceptedContract);

  //Check if the signed contract is valid
  if (!signedContract.contractId) {
    console.log('Error signing offer: ', signedContract);
    process.exit(1);
  }
  const contractID = signedContract.contractId;

  //Check if the contract is in the Signed state
  assert(
    retry(async () => checkIfContractIsInState(contractID, 'Signed'), 15000),
    `Contract state is not updated in the Router Wallet to Signed`
  );

  const txID = await dlcManager.countersign_and_broadcast(JSON.stringify(signedContract));
  let blockchainHeightAtBroadcast = await getBlockchainHeight();
  console.log(`Broadcast funding transaction with TX ID: ${txID}`);

  //Fetching Funding TX Details to check if the broadcast was successful
  console.log('Fetching Funding TX Details');
  await fetchTxDetails(txID);

  //Waiting for funding transaction confirmations
  const confirmedBroadcastTransaction = await waitForConfirmations(blockchainHeightAtBroadcast, 6);
  if (confirmedBroadcastTransaction) {
    console.log('Funding transaction confirmed');
  }

  //Check if the contract is in the Confirmed state
  assert(
    retry(async () => checkIfContractIsInState(contractID, 'Confirmed'), 15000),
    `Contract state is not updated in the Router Wallet to Confirmed`
  );

  //Check if the balance decreased after broadcasting the funding transaction
  const balanceAfterFunding = await checkBalance(dlcManager, '[CONTRACT CONFIRMED]');
  assert(
    Number(balanceAfterFunding) < Number(startingBalance),
    'BTC Balance did not decrease after broadcasting Funding TX'
  );

  //Attesting to Events
  if (handleAttestors) {
    console.log('Attesting to Events');
    const attestations = await Promise.all(
      attestorList.map((attestorURL, index) =>
        attest(
          attestorURL,
          testUUID,
          successfulAttesting
            ? acceptorGetsAllOutcome
            : index === offererGetsAllOutcome
            ? offererGetsAllOutcome
            : acceptorGetsAllOutcome
        )
      )
    );
    console.log('Attestation received: ', attestations);
  }

  //Check if the contract is in the PreClosed state
  assert(
    retry(async () => checkIfContractIsInState(contractID, 'PreClosed'), 15000),
    `Contract state is not updated in the Router Wallet to PreClosed`
  );

  //Waiting for funding transaction confirmations
  blockchainHeightAtBroadcast = await getBlockchainHeight();
  const confirmedClosingTransaction = await waitForConfirmations(blockchainHeightAtBroadcast, 6);
  if (confirmedClosingTransaction) {
    console.log('Closing transaction confirmed');
  }

  //Check if the contract is in the Closed state
  assert(
    retry(async () => checkIfContractIsInState(contractID, 'Closed'), 15000),
    `Contract state is not updated in the Router Wallet to Closed`
  );

  //Check if the balance increased after closing the contract
  checkBalance(dlcManager, '[CONTRACT CLOSED]').then((balanceAfterClosing) => {
    assert(
      Number(balanceAfterFunding) + Number(acceptCollateral) === Number(balanceAfterClosing),
      'Balance after closing does not match the expected value'
    );
  });

  process.exit(0);
}

main();
