import { JsDLCInterface } from "dlc_protocol_wallet";

async function fetchOffer() {
    let body = {
        "uuid": "abc12345",
        "acceptCollateral": 100000,
        "offerCollateral": 0,
        "totalOutcomes": 100
    };

    // fetch post with body
    // use fetch library to do a post request to the offeror wallet
    return fetch('http://localhost:8085/offer', {
        method: 'post',
        body: JSON.stringify(body),
        headers: { 'Content-Type': 'application/json' },
    }).then(res => res.json());
}

// class BaseContract {
//     state,
//     temporaryContractId
// }
// class OfferedContract extends StatelessContract<BaseContract> {
//     state: ContractState.Offered
//     contractInfo: ContractInfo
//     fundOutputSerialId: number
//     feeRatePerVByte: number
//     contractMaturityBound: number
//     contractTimeOut: number
//     isOfferParty: false
// }

async function setup() {
    console.log("Setting up");
    const dlc_man = await JsDLCInterface.new();
    console.log("options: ", dlc_man.get_options());
    dlc_man.get_contracts().then((res) => console.log("contracts: ", res));
    var balance = 0;
    var offered_contract;
    var runonce = true;
    // call doWork every 5 seconds in a loop
    var _loopId = setInterval(() => {
        dlc_man.get_wallet_balance().then((bal) => balance = bal);
        console.log("Balance: " + balance);
        if (!offered_contract && balance > 0 && runonce) {
            runonce = false;

            fetchOffer().then((offer_json) => {
                console.log("offer_json: ", offer_json);


                dlc_man.accept_offer(JSON.stringify(offer_json), dlc_man.get_options().address).then((accepted_contract) => {

                    console.log("Got response from receive_offer_and_accept", accepted_contract);
                    fetch('http://localhost:8085/offer/accept', {
                        method: 'put',
                        body: JSON.stringify({
                            'acceptMessage': accepted_contract
                        }),
                        headers: { 'Content-Type': 'application/json' },
                    }).then(res => {
                        console.log("intermediary response: ", res);
                        return res.json();
                    }).then((signed_contract) => {

                        console.log("signed offer: ", signed_contract);
                        dlc_man.countersign_and_broadcast(JSON.stringify(signed_contract)).then((res) => {
                            console.log("Got response from sign_offer", res);
                        });

                    });
                });
            });
        }
    }, 5000);
}

setup();


// Optionally could set the UTXOs manually in the accept_offer function
const utxos = [
    {
        "txid": "56724d6eb9cb1fb98bde892687e0fffdaf6665f9301c801a3f198fe8d876e60d",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2434970,
            "block_hash": "00000000000043ea2eae6480760ce7aee70bfc2dd98f9252686fdfbb56aca57e",
            "block_time": 1684847842
        },
        "value": 295165
    },
    {
        "txid": "2a7845a07ea3e711da380e0eb36c4557bd63c75141ed09503fe36b6af7a1db78",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432194,
            "block_hash": "00000000000000089f340be46ce9ff7cd91c20fd3aa6640341e938902d01b891",
            "block_time": 1683634291
        },
        "value": 10000
    },
    {
        "txid": "2b3642f2b25bee93c79e5a64d22062cbeb4d3da10d131b9687e26b1459e34d42",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2434981,
            "block_hash": "000000000000c7088096892cb1d5d91ab8e315e3ef8414c97b173cfd2f4c4314",
            "block_time": 1684855461
        },
        "value": 11834
    },
    {
        "txid": "64b9377cd6c866486246738eddef0c23b1c9d09c790dd7169ef4c8a126f4f10a",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432400,
            "block_hash": "0000000000000013bbb80a199591bfdecad8e579c29dcc6235bfa6bb69be7576",
            "block_time": 1683814665
        },
        "value": 10000
    },
    {
        "txid": "a7dd41233c9e0eb993b6b1dbfd4e9f7d8983fbc1a49bf8ca47d3e4b11032ea3a",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2426590,
            "block_hash": "000000000001011655a3dfcc3934a973bb31667b476a967efbfb7062e4a08041",
            "block_time": 1680194148
        },
        "value": 6652
    },
    {
        "txid": "d2e8fc2a6b09f1148e23f50cc1f27396d1d0c6367723718d555cbfa4370974c6",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432407,
            "block_hash": "000000000000000460633bba2e5d7603547951c674af07baa19b7368b906a4ef",
            "block_time": 1683821976
        },
        "value": 10000
    },
    {
        "txid": "71cf13b149efb24a70ea8076268e0c8944045523d992b6de7fdf47fb2e45ce03",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2431889,
            "block_hash": "00000000e82d140e9f28af9f55cf12d28367ddd252e5973b94a08b8224fae375",
            "block_time": 1683305629
        },
        "value": 10000
    },
    {
        "txid": "be8b5dee362d228f802f3adf5b34ed296c1b35fe9a73b4445cf7ff5c3f907cac",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2427206,
            "block_hash": "0000000000000024e595c4bceb57ebea496eb2ade942f1ebc2ad5d38e993ba5c",
            "block_time": 1680524340
        },
        "value": 6974
    },
    {
        "txid": "b27cbd31ca8e8762588d21777450cff9b740ee17861bdecffedccec124977d14",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432408,
            "block_hash": "0000000000000005a18e7316fd7d759b1e98586a8a228dee5f04bb39b2ea2c73",
            "block_time": 1683822174
        },
        "value": 10000
    },
    {
        "txid": "87c9494cf5ac8cf9eaccd05d70b43c5cc5cfa3f859728a8683886208341c1b23",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432408,
            "block_hash": "0000000000000005a18e7316fd7d759b1e98586a8a228dee5f04bb39b2ea2c73",
            "block_time": 1683822174
        },
        "value": 10000
    },
    {
        "txid": "2f708526f68703d191e2f9c357455c05a42f7c8dd7eab2f3d3f0a45b1ab79d30",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432210,
            "block_hash": "00000000000000193573fc74171fb7dfd41f149f88161699ebd1059c6523617e",
            "block_time": 1683646875
        },
        "value": 10000
    },
    {
        "txid": "1eec9d5e3b872b00632669111714df63ded35e2283044cbc4968ffc6fab7486a",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2432296,
            "block_hash": "0000000000000000cd957893cd52640e0233309b5e83d76016f179b0ab8542ab",
            "block_time": 1683719331
        },
        "value": 10000
    },
    {
        "txid": "1a97d9ae49d48d3abcdbe1fcb3fed313eff601c321f2b0718336a803b92a4ef8",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2434981,
            "block_hash": "000000000000c7088096892cb1d5d91ab8e315e3ef8414c97b173cfd2f4c4314",
            "block_time": 1684855461
        },
        "value": 6000
    },
    {
        "txid": "a747427f031a7626e1ab01fb5e28ad7986358a40074ef08607919940e4f76854",
        "vout": 0,
        "status": {
            "confirmed": true,
            "block_height": 2428861,
            "block_hash": "00000000055c129146966449c55f8a314bcdc1f863195a63f6d1b0b9219579b4",
            "block_time": 1681469214
        },
        "value": 10000
    },
    {
        "txid": "962cb9d2274d2c7979ee75d93ee44ecb47163232e2c167dcc8563e4988b08102",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427689,
            "block_hash": "000000000000001f6fde5e7eb9f0597c4dd3465f5371c5e68314b471063ff668",
            "block_time": 1680796673
        },
        "value": 10000
    },
    {
        "txid": "3d174276775887f115785735c0c3a50ea6092e3089f73dff1b0726b27aa29865",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427652,
            "block_hash": "00000000621b6ae469e1d4394b0c37c15130838aad2d0b2463bf8fde6010f173",
            "block_time": 1680778092
        },
        "value": 3282
    },
    {
        "txid": "d5d65aecbdaf139f4dfc27f53b0d4e508c1967a237d9db5ed534e8fb5da96164",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427689,
            "block_hash": "000000000000001f6fde5e7eb9f0597c4dd3465f5371c5e68314b471063ff668",
            "block_time": 1680796673
        },
        "value": 10000
    },
    {
        "txid": "af33b61a670a0988885540f2fa77773b8c17defdd355a356fb42042ac211bd0a",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427689,
            "block_hash": "000000000000001f6fde5e7eb9f0597c4dd3465f5371c5e68314b471063ff668",
            "block_time": 1680796673
        },
        "value": 10000
    },
    {
        "txid": "da454c5aedaa49bc8ab4a56071c3cabe121a6aafe63fd62288275cacbe2ba216",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2428471,
            "block_hash": "0000000000002f762534e2b821dbd4a8df0fa5e5c894dc585d16258041b1ac27",
            "block_time": 1681219672
        },
        "value": 5282
    },
    {
        "txid": "23574aee08bc634dfe59fc7a83eace671607050e7c0ef68d65958f45e1314672",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427690,
            "block_hash": "0000000000000025b8d27c1154caf02ce5cf9867a1eaf52d1c36407f1c902e59",
            "block_time": 1680797278
        },
        "value": 10000
    },
    {
        "txid": "70e57eaefe0af6411733f55bd833038e5e06751ffa76a06b135f65e7d608f060",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427635,
            "block_hash": "0000000000000000f01e74bb98b4505d5b39aff03e2e2b3923ecb65472b20361",
            "block_time": 1680768802
        },
        "value": 3282
    },
    {
        "txid": "6aecdef583264db4c97f26aec2725a6e0f93ac22486e3da70725cb9069cdec84",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427218,
            "block_hash": "0000000000000013d6fbb39acc48165e2242fe73ddf222f8fef424f04089ec16",
            "block_time": 1680531685
        },
        "value": 10000
    },
    {
        "txid": "b2881a314def1c6d28f3f128d3bda3daab6550750b4aa8aaca481bf76f758e3c",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427652,
            "block_hash": "00000000621b6ae469e1d4394b0c37c15130838aad2d0b2463bf8fde6010f173",
            "block_time": 1680778092
        },
        "value": 2282
    },
    {
        "txid": "7c0803ca940405db5392151d46586a44c0d81040135ebf2e21bbf655cd2ff0ec",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427689,
            "block_hash": "000000000000001f6fde5e7eb9f0597c4dd3465f5371c5e68314b471063ff668",
            "block_time": 1680796673
        },
        "value": 10000
    },
    {
        "txid": "9955dda458e9eae26613300f7ab51522efb511fbc19c876e34862144c2a20ada",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427212,
            "block_hash": "000000000000527f643f21efd5bb5a1736aaeb585d976b954c9c184458b0552f",
            "block_time": 1680529560
        },
        "value": 10000
    },
    {
        "txid": "9a201c975d5e8291c3989ffafff3067ff75ebdb5604e875c0bb333f9fb6f3389",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427355,
            "block_hash": "000000000000a4c09a5c84d826e5a26ebe690a90dded9b4b308284d991d8dd52",
            "block_time": 1680612200
        },
        "value": 1000
    },
    {
        "txid": "86f59d5804eefbbaed06b2228f373e2034626c633a3cdc34dbbdb802f17ac813",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427222,
            "block_hash": "000000000000001d6b6e6df5b85e6bfe25f56964ec5bfe33884d70e697be4650",
            "block_time": 1680532652
        },
        "value": 10000
    },
    {
        "txid": "aa90de28e06d431f7221c1b2064c5225f09504e40ef46fb71c10cffefc4e5f9f",
        "vout": 1,
        "status": {
            "confirmed": true,
            "block_height": 2427212,
            "block_hash": "000000000000527f643f21efd5bb5a1736aaeb585d976b954c9c184458b0552f",
            "block_time": 1680529560
        },
        "value": 10000
    },
    {
        "txid": "d4bfbc6e2fbf997162566b705ee98defe12ec436a944dfe1b2591f00b0f2854c",
        "vout": 2,
        "status": {
            "confirmed": true,
            "block_height": 2427206,
            "block_hash": "0000000000000024e595c4bceb57ebea496eb2ade942f1ebc2ad5d38e993ba5c",
            "block_time": 1680524340
        },
        "value": 6000
    }
]
