import { JsDLCInterface } from "dlc_protocol_wallet";

const testWalletPrivateKey =
  "f8ec31c12b6d014249935f2cb76b543b442ac2325993b44cbed4cdf773fbc8df";
const testWalletAddress = "bcrt1qatfjgacgqaua975r0cnsqtl09td8636jm3vnp0";

const bitcoinNetwork = "regtest";
const bitcoinNetworkURL = "https://dev-oracle.dlc.link/electrs";

const protocolWalletURL = "http://localhost:8085";

// const oracleURLs = ["http://localhost:8801", "http://localhost:8802"];
const oracleURLs = ["http://localhost:8801"];
const joinedOracleURLs = ["http://localhost:8801"];

// const joinedOracleURLs = oracleURLs.join(",");

async function fetchOfferFromProtocolWallet(oracleUrls) {
  let body = {
    uuid: "0xa2876fa089db7c4102157b909e31013f00b9558944bac010ecf49ef87155d0ce",
    acceptCollateral: 10000,
    offerCollateral: 0,
    totalOutcomes: 100,
    // "oraclesUrls": oracleUrls,
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

async function go() {
  console.log("DLC WASM Wallet Test");

  // creates a new instance of the JsDLCInterface
  const dlcManager = await JsDLCInterface.new(
    testWalletPrivateKey,
    testWalletAddress,
    bitcoinNetwork,
    bitcoinNetworkURL,
    joinedOracleURLs
  );

  console.log("DLC Manager Interface Options: ", dlcManager.get_options());

  async function waitForBalance(dlcManager) {
    let balance = 0;
    while (balance <= 0) {
      balance = await dlcManager.get_wallet_balance();
      console.log("DLC Wasm Wallet Balance: " + balance);
      await new Promise((resolve) => setTimeout(resolve, 5000));
    }
    return balance;
  }

  waitForBalance(dlcManager).then((balance) => {
    runDLCFlow(dlcManager);
  });
}

async function runDLCFlow(dlcManager) {
  console.log("Starting DLC flow");

  const offer_json = await fetchOfferFromProtocolWallet(
    dlcManager.get_options().oracle_urls
  );
  console.log("Offer (JSON): ", offer_json);

  const accepted_contract = await dlcManager.accept_offer(
    JSON.stringify(offer_json)
  );
  console.log("Accepted Contract:", accepted_contract);

  const signed_contract = await sendAcceptedOfferToProtocolWallet(
    accepted_contract
  );
  console.log("Signed Contract: ", signed_contract);

  const tx_id = await dlcManager.countersign_and_broadcast(
    JSON.stringify(signed_contract)
  );
  console.log(`Broadcast funding transaction with TX ID: ${tx_id}`);

  const contracts = await dlcManager.get_contracts();
  console.log("Contracts: ", contracts);
}

go();
