/* eslint-disable no-unused-vars */
import dotenv from 'dotenv';
dotenv.config();
import { JsDLCInterface } from '../node_modules/wasm-wallet/dlc_tools.js';
import fetch from 'cross-fetch';
import config from './config.js';
import setupPolyfills from './polyfills.js';

const DEFAULT_WAIT_TIME = 60000;
const BLOCK_TIME = 5000;
setupPolyfills();

const {
  testWalletPrivateKey,
  testWalletAddress,
  bitcoinNetwork,
  bitcoinNetworkURL,
  protocolWalletURL,
  attestorList,
  storageApiUrl,
} = config;

async function fetchOfferFromProtocolWallet(uuid, overrides = {}) {
  let body = {
    uuid,
    // acceptCollateral,
    // refundDelay: 86400 * 7,
  };

  body = { ...body, ...overrides };

  console.log('Offer body: ', body);

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

async function retry(checkFunction, timeoutTime) {
  let timeRemaining = timeoutTime;
  while (timeRemaining > 0) {
    const result = await checkFunction();
    if (result) return true;
    await new Promise((resolve) => setTimeout(resolve, 2000));
    timeRemaining -= 2000;
  }
  return false;
}

function assert(predicate, message) {
  if (!predicate) {
    console.log(message);
    process.exit(1);
  }
}

async function waitForConfirmations(blockchainHeightAtBroadcast, targetConfirmations) {
  const url = `${process.env.ELECTRUM_API_URL}/blocks/tip/height`;
  let currentBlockchainHeight = blockchainHeightAtBroadcast;
  while (Number(currentBlockchainHeight) - Number(blockchainHeightAtBroadcast) < targetConfirmations) {
    await new Promise((resolve) => setTimeout(resolve, BLOCK_TIME));
    currentBlockchainHeight = await (await fetch(url)).json();
    console.log(
      `[IT] Confirmations: ${
        Number(currentBlockchainHeight) - Number(blockchainHeightAtBroadcast)
      } / ${targetConfirmations}`
    );
  }
  return true;
}

async function checkIfContractIsInState(contractID, state) {
  const routerWalletInfo = await (await fetch(`${protocolWalletURL}/info`)).json();
  let result = routerWalletInfo.contracts[state].includes(contractID);
  console.log('[IT] Is contract ID: ', contractID, ' in state: ', state, '? ', result);
  return result;
}

async function getBlockchainHeight() {
  const url = `${process.env.ELECTRUM_API_URL}/blocks/tip/height`;
  const currentBlockchainHeight = await (await fetch(url)).json();
  return currentBlockchainHeight;
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

async function setupDLC(dlcManager, uuid, time, overrides = {}) {
  //Fetching Offer
  console.log('Fetching Offer from Protocol Wallet');
  const offerResponse = await fetchOfferFromProtocolWallet(uuid, { ...overrides });

  //Check if the offer is valid
  if (!offerResponse.temporaryContractId) {
    console.error('[IT] Error fetching offer from protocol wallet: ', offerResponse);
    process.exit(1);
  }

  //Accepting Offer
  const acceptedContract = await dlcManager.accept_offer(JSON.stringify(offerResponse));
  const parsedResponse = JSON.parse(acceptedContract);

  //Check if the accepted contract is valid
  if (!parsedResponse.protocolVersion) {
    console.log('[IT] Error accepting offer: ', parsedResponse);
    process.exit(1);
  }

  //Sending Accepted Offer to Protocol Wallet
  const signedContract = await sendAcceptedOfferToProtocolWallet(acceptedContract);

  //Check if the signed contract is valid
  if (!signedContract.contractId) {
    console.log('[IT] Error signing offer: ', signedContract);
    process.exit(1);
  }
  const contractID = signedContract.contractId;

  //Check if the contract is in the Signed state
  assert(
    await retry(async () => checkIfContractIsInState(contractID, 'Signed'), DEFAULT_WAIT_TIME),
    `[IT] Contract state is not updated in the Router Wallet to Signed`
  );

  const txID = await dlcManager.countersign_and_broadcast(JSON.stringify(signedContract));
  let blockchainHeightAtBroadcast = await getBlockchainHeight();
  console.log(`[IT] Broadcast funding transaction with TX ID: ${txID}`);

  //Fetching Funding TX Details to check if the broadcast was successful
  console.log('[IT] Fetching Funding TX Details');
  await fetchTxDetails(txID);

  //Waiting for funding transaction confirmations
  let confirmedBroadcastTransaction = await waitForConfirmations(blockchainHeightAtBroadcast, 1);
  if (confirmedBroadcastTransaction) {
    console.log('[IT] Funding transaction confirmed');
  }

  return { blockchainHeightAtBroadcast, contractID };
}

async function main() {
  //Creating DLC Manager Interface
  const dlcManager = await JsDLCInterface.new(
    testWalletPrivateKey,
    testWalletAddress,
    bitcoinNetwork,
    bitcoinNetworkURL,
    storageApiUrl
  );

  console.log('[IT] Starting DLC Integration Tests');

  // Test the happy path
  const testUUID = process.env.UUID || `test${Math.random().toString(36).slice(2)}`;
  let setupDetails1 = await setupDLC(dlcManager, testUUID);

  //Waiting for funding transaction confirmations
  let confirmedBroadcastTransaction = await waitForConfirmations(setupDetails1.blockchainHeightAtBroadcast, 6);
  if (confirmedBroadcastTransaction) {
    console.log('[IT] Funding transaction confirmed');
  }

  //Check if the contract is in the Confirmed state
  assert(
    await retry(async () => checkIfContractIsInState(setupDetails1.contractID, 'Confirmed'), DEFAULT_WAIT_TIME),
    `[IT] Contract state is not updated in the Router Wallet to Confirmed`
  );
}
