use cucumber::{given, when, then, Parameter, World};
use dlc_clients::{AcceptMessage, ApiResult, OfferRequest, OracleBackendClient, WalletBackendClient};
use derive_more::{Deref, FromStr};
use tokio::runtime::Runtime;

#[derive(Deref, FromStr, Parameter)]
#[param(regex = r"\d+", name = "u64")]
struct CustomU64(u64);

#[derive(Debug, Default, World)]
pub struct DlcLinkWorld {
    wallet_client: WalletBackendClient,
    oracle_client: OracleBackendClient,
    collected_responses: Vec<ApiResult>,
}

#[given(expr="a wallet backend client with address {word}")]
fn create_wallet_client(world: &mut DlcLinkWorld, address: String) {
    world.wallet_client = WalletBackendClient::new(address);
}

#[given(expr="an oracle backend client with address {word}")]
fn create_oracle_client(world: &mut DlcLinkWorld, address: String) {
    world.oracle_client = OracleBackendClient::new(address);
}

#[when(expr="accept message: {word}")]
async fn wallet_accept_message(world: &mut DlcLinkWorld, accept_message: String) {
    let accept_msg_request = AcceptMessage {
        accept_message: accept_message.to_string()
    };
    let _res = world.wallet_client.put_accept(accept_msg_request);
}

#[when(expr="creating an offer request with uuid {word}, accept_collateral: {u64} and offer_collateral: {u64}")]
async fn create_offer(world: &mut DlcLinkWorld, uuid: String, accept_collateral: CustomU64, offer_collateral: CustomU64) {
    //{
    //    "uuid": "0xfdc34ee06024ad476e18fff8a80faa5d14dc427cfff3131642c77eaff6bc1d9a",
    //    "acceptCollateral": 100000000,
    //    "offerCollateral": 10000,
    //    "totalOutcomes": 1
    //}
    let offer_request = OfferRequest {
        uuid: uuid.to_string(),
        accept_collateral: *accept_collateral,
        offer_collateral: *offer_collateral,
        total_outcomes: 1,
    };
    let res = world.wallet_client.post_offer_and_accept(offer_request);
    world.collected_responses.push(res.await.unwrap());
}

#[when(expr="creating a new oracle event with uuid {word}")]
async fn create_event(world: &mut DlcLinkWorld, uuid: String) {
    let res = world.oracle_client.create_event(uuid.to_string());
    world.collected_responses.push(res.await.unwrap());
}

#[when(expr="getting an attestation with uuid {word} and outcome: {word}")]
async fn get_attest(world: &mut DlcLinkWorld, uuid: String, outcome: String) {
    let res = world.oracle_client.get_attestation(uuid.to_string(), outcome.to_string());
    world.collected_responses.push(res.await.unwrap());
}

#[then(expr = "expected status code is {u64}")]
fn expected_offer_result(world: &mut DlcLinkWorld, status_code: CustomU64) {
    let api_res = world.collected_responses.get(0).unwrap();
    assert_eq!(*status_code, api_res.status as u64);
}

fn main() {
    Runtime::new().unwrap().block_on(DlcLinkWorld::run("tests/features"));
}