/* eslint-disable no-unused-vars */
import dotenv from 'dotenv';
dotenv.config();
import { JsDLCInterface } from '../node_modules/wasm-wallet/dlc_tools.js';
import fetch from 'cross-fetch';
import config from './config.js';
import setupPolyfills from './polyfills.js';
import { bytesToHex, hexToBytes } from '@noble/hashes/utils';
import * as btc from '@scure/btc-signer';

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

const successfulAttesting = process.env.SUCCESSFUL_ATTESTING == 'true';

const acceptorGetsAllOutcome = 0;
const offererGetsAllOutcome = 100;
const protocol_fee_percent = 0.01;
const btcFeeBasisPoints = protocol_fee_percent * 10000;
const btcFeeRecipient = 'bcrt1qvgkz8m4m73kly4xhm28pcnv46n6u045lfq9ta3';

// NOTE: we no longer send this amount in the offer, but it is hardcoded in the WBI testmode as well.
// ../wallet/wallet-blockchain-interface/src/http/public-server/routes.ts
// If you change it, you need to change it in both places.
const acceptCollateral = 1000000;

async function createEvent(attestorURL, uuid, time = '') {
  try {
    let url = `${attestorURL}/create-announcement/${uuid}`;
    if (time) {
      url = `${url}?time=${time}`;
    }
    console.log('Creating event: ', url);
    const response = await fetch(url);
    const event = await response.json();
    return event;
  } catch (error) {
    console.error('Error creating event: ', error);
    process.exit(1);
  }
}

async function createPsbtEvent(attestorURL, uuid, psbt1, psbt2, mintAddress) {
  try {
    let url = `${attestorURL}/create-psbt-event`;
    console.log('Creating event: ', url);
    const response = await fetch(url, {
      method: 'POST',
      body: JSON.stringify({
        uuid,
        psbt1,
        psbt2,
        mintAddress,
      }),
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
    });

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

async function fetchOfferFromProtocolWallet(uuid, overrides = {}) {
  let body = {
    uuid,
    acceptCollateral,
    refundDelay: 86400 * 7,
    btcFeeRecipient: btcFeeRecipient,
    btcFeeBasisPoints: btcFeeBasisPoints,
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

async function checkBalance(dlcManager, action) {
  const balance = await dlcManager.get_wallet_balance();
  console.log(`[IT] DLC Wasm Wallet Balance at ${action}: ` + balance);
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

async function setupDLC(dlcManager, uuid, time, overrides = {}) {
  let startingBalance = await checkBalance(dlcManager, '[START TEST]');

  if (process.env.HANDLE_ATTESTORS == 'true') {
    //Creating Events
    console.log('Creating Event');
    const events = await Promise.all(attestorList.map((attestorURL) => createEvent(attestorURL, uuid, time)));
  }

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
  let fund_tx_details = await fetchTxDetails(txID);

  //Check if the funding transaction has the protocol_fee output
  const vouts = fund_tx_details.vout;
  assert(
    vouts.some(
      (vout) => vout.value === acceptCollateral * protocol_fee_percent && vout.scriptpubkey_address === btcFeeRecipient
    ),
    '[IT] Funding transaction does not have the protocol_fee output'
  );

  //Waiting for funding transaction confirmations
  let confirmedBroadcastTransaction = await waitForConfirmations(blockchainHeightAtBroadcast, 1);
  if (confirmedBroadcastTransaction) {
    console.log('[IT] Funding transaction confirmed');
  }

  //Check if the balance decreased after broadcasting the funding transaction and waiting 1 confirmation
  let balanceAfterFunding = await checkBalance(dlcManager, '[1 CONFIRMATION]');
  assert(
    Number(balanceAfterFunding) < Number(startingBalance),
    '[IT] BTC Balance did not decrease after broadcasting Funding TX. Expected: ' +
      Number(balanceAfterFunding) +
      ' should be less than ' +
      Number(startingBalance)
  );
  return { blockchainHeightAtBroadcast, contractID };
}

async function create_attestations_for_uuid(uuid) {
  //Attesting to Events
  console.log('[IT] Attesting to Events');
  let attestations = await Promise.all(
    attestorList.map((attestorURL, index) =>
      attest(
        attestorURL,
        uuid,
        successfulAttesting
          ? acceptorGetsAllOutcome
          : index === offererGetsAllOutcome
            ? offererGetsAllOutcome
            : acceptorGetsAllOutcome
      )
    )
  );
  console.log('[IT] Attestation 1 received: ', attestations);
}

async function verify_closed_and_balance_returned(dlcManager, contractID, uuid) {
  let startingBalance = await checkBalance(dlcManager, '[ALL FUNDED BALANCE]');
  await create_attestations_for_uuid(uuid);
  //Wait for the contract to move into the PreClosed state
  assert(
    await retry(async () => checkIfContractIsInState(contractID, 'PreClosed'), DEFAULT_WAIT_TIME),
    `[IT] Contract state is not updated in the Router Wallet to PreClosed`
  );

  let blockchainHeightAtBroadcast = await getBlockchainHeight();
  //Waiting for closing transaction confirmations
  let confirmedClosingTransaction = await waitForConfirmations(blockchainHeightAtBroadcast, 6);
  if (confirmedClosingTransaction) {
    console.log('[IT] Closing transaction confirmed');
  }

  let balanceAfterClosing = await checkBalance(dlcManager, '[CONTRACT CLOSED]');
  let desiredBalace = Number(startingBalance) + Number(acceptCollateral);
  assert(
    desiredBalace === Number(balanceAfterClosing),
    '[IT] Balance after closing does not match the expected value. Expected: ' +
      desiredBalace +
      ' Actual: ' +
      Number(balanceAfterClosing)
  );

  //Check if the contract is in the Closed state
  assert(
    await retry(async () => checkIfContractIsInState(contractID, 'Closed'), DEFAULT_WAIT_TIME),
    `[IT] Contract state is not updated in the Router Wallet to Closed`
  );
}

async function verify_refund_tx(dlcManager, contractID) {
  let startingBalance = await checkBalance(dlcManager, '[ALL FUNDED BALANCE]');
  //Check if the contract is in the PreClosed state
  assert(
    await retry(async () => checkIfContractIsInState(contractID, 'Refunded'), DEFAULT_WAIT_TIME),
    `[IT] Contract state is not updated in the Router Wallet to Refunded`
  );

  let blockchainHeightAtBroadcast = await getBlockchainHeight();
  //Waiting for closing transaction confirmations
  let confirmedClosingTransaction = await waitForConfirmations(blockchainHeightAtBroadcast, 6);
  if (confirmedClosingTransaction) {
    console.log('[IT] Closing transaction confirmed');
  }

  let balanceAfterClosing = await checkBalance(dlcManager, '[CONTRACT CLOSED]');
  let desiredBalace = Number(startingBalance) + Number(acceptCollateral);
  console.log('[IT] comparing balance after closing', desiredBalace, Number(balanceAfterClosing));
  assert(
    desiredBalace === Number(balanceAfterClosing),
    '[IT] Balance after closing does not match the expected value. Expected: ' +
      desiredBalace +
      ' Actual: ' +
      Number(balanceAfterClosing)
  );
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

  await checkBalance(dlcManager, '[STARTING BALANCE]');
  console.log('[IT] Starting DLC Integration Tests');

  // //Start first test
  // console.log('[IT] ##################### STARTING HAPPY PATH TEST #####################');
  // // Test the happy path
  // const testUUID = process.env.UUID || `test${Math.random().toString(36).slice(2)}`;
  // let setupDetails1 = await setupDLC(dlcManager, testUUID);

  // //Start second test
  // // console.log('[IT] ##################### STARTING SECOND TEST #####################');
  // // This is a placeholder, feel free to overwrite this test
  // // const testUUID2 = process.env.UUID || `test${Math.random().toString(36).slice(2)}`;
  // // let setupDetails2 = await setupDLC(dlcManager, testUUID2);

  // //Waiting for funding transaction confirmations
  // let confirmedBroadcastTransaction = await waitForConfirmations(setupDetails1.blockchainHeightAtBroadcast, 6);
  // if (confirmedBroadcastTransaction) {
  //   console.log('[IT] Funding transaction confirmed');
  // }

  // //Check if the contract is in the Confirmed state
  // assert(
  //   await retry(async () => checkIfContractIsInState(setupDetails1.contractID, 'Confirmed'), DEFAULT_WAIT_TIME),
  //   `[IT] Contract state is not updated in the Router Wallet to Confirmed`
  // );

  // // //Waiting for funding transaction confirmations
  // // confirmedBroadcastTransaction = await waitForConfirmations(setupDetails2.blockchainHeightAtBroadcast, 6);
  // // if (confirmedBroadcastTransaction) {
  // //   console.log('Funding transaction confirmed');
  // // }

  // // //Check if the contract is in the Confirmed state
  // // assert(
  // //   await retry(async () => checkIfContractIsInState(setupDetails2.contractID, 'Confirmed'), DEFAULT_WAIT_TIME),
  // //   `Contract state is not updated in the Router Wallet to Confirmed`
  // // );

  // // ----------------------------------------------

  // await checkBalance(dlcManager, '[ALL FUNDED BALANCE]');

  // console.log(`[IT] Closing DLC for ${testUUID} created`);
  // await verify_closed_and_balance_returned(dlcManager, setupDetails1.contractID, testUUID);

  // // console.log(`Closing DLC for ${testUUID2} created`);
  // // await verify_closed_and_balance_returned(dlcManager, setupDetails2.contractID, testUUID2);

  // // --- Refund tests

  // //Start third test
  // console.log('[IT] ##################### STARTING REFUND TEST #####################');
  // const testUUID3 = process.env.UUID || `test${Math.random().toString(36).slice(2)}`;
  // // Create a DLC that will refund. Attestation maturity is 20 seconds in the future, and refund delay is 20 second after that.
  // let setupDetails3 = await setupDLC(dlcManager, testUUID3, new Date().getTime() + 20000, { refundDelay: 20 });

  // //Waiting for funding transaction confirmations
  // confirmedBroadcastTransaction = await waitForConfirmations(setupDetails3.blockchainHeightAtBroadcast, 6);
  // if (confirmedBroadcastTransaction) {
  //   console.log('[IT] Funding transaction confirmed');
  // }

  // //Check if the contract is in the Confirmed state
  // assert(
  //   await retry(async () => checkIfContractIsInState(setupDetails3.contractID, 'Confirmed'), DEFAULT_WAIT_TIME),
  //   `[IT] Contract state is not updated in the Router Wallet to Confirmed`
  // );

  // await verify_refund_tx(dlcManager, setupDetails3.contractID);

  console.log('[IT] ##################### STARTING PSBT TEST #####################');

  const usersNativeSegwitAddress = {
    symbol: 'BTC',
    type: 'p2wpkh',
    address: 'bcrt1qk5q0takwdva20adgw8zf4vy07w9529gpfkrv6v',
    publicKey: '02ab6fdbc88fb3d52dc83e5e92f27d26bfc3d161d24b50fa299f3c571f0d5a3220',
    derivationPath: "m/84'/1'/2'/0/0",
  };
  const usersTRAddress = {
    symbol: 'BTC',
    type: 'p2tr',
    address: 'bcrt1pa8hxt6r2gkc8d5thzfrw7gyrqlv354rdy4k05ylkvf6nadnhg8xsf3w69n',
    publicKey: '039f4157be1fe22cb293558b394c6ae237d670b0ac577b31e71091fe6efa789c42',
    tweakedPublicKey: '9f4157be1fe22cb293558b394c6ae237d670b0ac577b31e71091fe6efa789c42',
    derivationPath: "m/86'/1'/2'/0/0",
  };

  // get the unspent utxos from an esplora api
  const esploraUtxos = await (
    await fetch(`${process.env.ELECTRUM_API_URL}/address/${usersNativeSegwitAddress.address}/utxo`)
  ).json();

  const regtest = {
    bech32: 'bcrt',
    wif: 0xef,
    bip32: {
      public: 0x043587cf,
      private: 0x04358394,
    },
    pubKeyHash: 0x6f,
    scriptHash: 0xc4,
  };

  const spend = btc.p2wpkh(usersNativeSegwitAddress.publicKey, regtest);

  const utxos = await Promise.all(
    esploraUtxos.map(async (utxo) => {
      const txHex = await (await fetch(`${process.env.ELECTRUM_API_URL}/tx/${utxo.txid}/hex`)).text();
      return {
        ...spend,
        txid: utxo.txid,
        index: utxo.vout,
        value: utxo.value,
        nonWitnessUtxo: hexToBytes(txHex),
        // script: utxo.scriptpubkey, //do i need to handle when it is witness? how?
      };
    })
  );
  const outputs = [
    { address: 'bcrt1qvgkz8m4m73kly4xhm28pcnv46n6u045lfq9ta3', amount: 50_000n }, // amount in satoshi
  ];

  const selected = btc.selectUTXO(utxos, outputs, 'default', {
    changeAddress: 'bcrt1qk5q0takwdva20adgw8zf4vy07w9529gpfkrv6v', // required, address to send change
    feePerByte: 2n, // require, fee per vbyte in satoshi
    bip69: true, // lexicographical Indexing of Transaction Inputs and Outputs
    createTx: true, // create tx with selected inputs/outputs
    network: regtest,
  });
  const { tx } = selected;
  const psbt = tx.toPSBT();
  const psbtToSign = { hex: bytesToHex(psbt), broadcast: false };

  const events = await Promise.all(
    attestorList.map((attestorURL) =>
      createPsbtEvent(attestorURL, 'testuuid', bytesToHex(psbt), bytesToHex(psbt), 'myMintAddress')
    )
  );

  console.log('##############################################');
  console.log('DLC Integration Test Completed Successfully!');
  console.log(
    `███████╗██╗    ██╗███████╗███████╗████████╗
██╔════╝██║    ██║██╔════╝██╔════╝╚══██╔══╝
███████╗██║ █╗ ██║█████╗  █████╗     ██║
╚════██║██║███╗██║██╔══╝  ██╔══╝     ██║
███████║╚███╔███╔╝███████╗███████╗   ██║
╚══════╝ ╚══╝╚══╝ ╚══════╝╚══════╝   ╚═╝
`
  );

  process.exit(0);
}

main();
