import { Attestor } from "attestor";

async function go() {
  const attestor = await Attestor.new();
  await attestor.create_event("ashdlakshd", "2023-10-08T13:48:00Z");
}

go();

// import { JsDLCInterface } from "dlc_protocol_wallet";

// async function fetchOfferFromProtocolWallet() {
//     let body = {
//         "uuid": "abc12345",
//         "acceptCollateral": 100000,
//         "offerCollateral": 0,
//         "totalOutcomes": 100
//     };

//     return fetch('http://localhost:8085/offer', {
//         method: 'post',
//         body: JSON.stringify(body),
//         headers: { 'Content-Type': 'application/json' },
//     }).then(res => res.json());
// }

// async function sendAcceptedOfferToProtocolWallet(accepted_offer) {
//     return fetch('http://localhost:8085/offer/accept', {
//         method: 'put',
//         body: JSON.stringify({
//             'acceptMessage': accepted_offer
//         }),
//         headers: { 'Content-Type': 'application/json' },
//     }).then(res => res.json());
// }

// const key = "f8ec31c12b6d014249935f2cb76b543b442ac2325993b44cbed4cdf773fbc8df";
// const address = "bcrt1qatfjgacgqaua975r0cnsqtl09td8636jm3vnp0";

// async function go() {
//     console.log("DLC WASM test let's go");

//     // Create a new dlc manager interface
//     const dlc_man = await JsDLCInterface.new(key, address, "regtest", "https://dev-oracle.dlc.link/electrs", "https://dev-oracle.dlc.link/oracle");
//     console.log("dlc manager interface options: ", dlc_man.get_options());

//     var balance = 0;
//     // use a setInterval to wait for the balance to be > 0
//     var loopId = setInterval(() => {
//         if (balance > 0) {
//             clearInterval(loopId);

//             runDLCFlow(dlc_man);
//         }
//         dlc_man.get_wallet_balance().then((bal) => {
//             balance = bal;
//             console.log("Balance: " + balance);
//         });
//     }, 5000);
// }

// async function runDLCFlow(dlc_man) {
//     console.log("Starting DLC flow");

//     const offer_json = await fetchOfferFromProtocolWallet();
//     console.log("offer_json: ", offer_json);

//     const accepted_contract = await dlc_man.accept_offer(JSON.stringify(offer_json), dlc_man.get_options().address)
//     console.log("Got response from receive_offer_and_accept", accepted_contract);

//     const signed_contract = await sendAcceptedOfferToProtocolWallet(accepted_contract);
//     console.log("signed offer: ", signed_contract);

//     const tx_id = await dlc_man.countersign_and_broadcast(JSON.stringify(signed_contract))
//     console.log(`Broadcast DLC with tx-id ${tx_id}`);

//     const contracts = await dlc_man.get_contracts();
//     console.log("Got contracts: ", contracts);
// }

// go();
