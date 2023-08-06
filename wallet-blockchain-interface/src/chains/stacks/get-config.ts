import {
    callReadOnlyFunction,
    cvToValue,
    parsePrincipalString,
    ContractPrincipal,
    SignedContractCallOptions,
    contractPrincipalCV,
    addressToString,
    makeContractCall,
    broadcastTransaction,
    NonFungibleConditionCode,
    bufferCV,
    createAssetInfo,
    makeContractNonFungiblePostCondition,
    stringAsciiCV,
} from '@stacks/transactions';
import type { TxBroadcastResult } from '@stacks/transactions';
import { ConfigSet } from '../../config/models.js';
import { WrappedContract } from '../shared/models/wrapped-contract.interface.js';
import { hexToBytes, uuidToCV } from './helper-functions.js';
import { StacksMainnet, StacksMocknet, StacksNetwork, StacksTestnet } from '@stacks/network';
import { getEnv } from '../../config/read-env-configs.js';
import { NFTHoldingsData } from '../shared/models/nft-holdings-data.interface.js';

async function getCallbackContract(uuid: string, contractName: string, deployer: string, network: StacksNetwork) {
    const functionName = 'get-callback-contract';
    const txOptions = {
        contractAddress: deployer,
        contractName: contractName,
        functionName: functionName,
        functionArgs: [uuidToCV(uuid)],
        senderAddress: deployer,
        network: network,
    };
    const transaction: any = await callReadOnlyFunction(txOptions);
    const callbackContract = cvToValue(transaction.value);
    console.log(`Callback contract for uuid: '${uuid}':`, callbackContract);
    return parsePrincipalString(callbackContract) as ContractPrincipal;
}

async function getAllAttestors(
    deployer: string,
    contractName: string,
    nftName: string,
    api_base_extended: string
): Promise<NFTHoldingsData> {
    const getAttestorNFTsURL = `${api_base_extended}/tokens/nft/holdings?asset_identifiers=${deployer}.${contractName}::${nftName}&principal=${deployer}.${contractName}`;
    console.log(`[Stacks] Loading registered attestors from ${getAttestorNFTsURL}...`);
    try {
        const response = await fetch(getAttestorNFTsURL);
        return (await response.json()) as NFTHoldingsData;
    } catch (err) {
        console.error(err);
        throw err;
    }
}

export default async (config: ConfigSet): Promise<WrappedContract> => {
    const adminKey = getEnv('PRIVATE_KEY');
    let api_base_extended: string;
    const contractName = 'dlc-manager-v1';
    const dlcNFTName = `open-dlc`;
    const attestorNFTName = 'dlc-attestor';

    let network: StacksNetwork;
    let deployer: string;

    switch (config.chain) {
        case 'STACKS_MAINNET':
            network = new StacksMainnet();
            deployer = '';
            api_base_extended = 'https://api.hiro.so/extended/v1';
            break;
        case 'STACKS_TESTNET':
            network = new StacksTestnet();
            deployer = 'ST1JHQ5GPQT249ZWG6V4AWETQW5DYA5RHJB0JSMQ3';
            api_base_extended = 'https://api.testnet.hiro.so/extended/v1';
            break;
        case 'STACKS_MOCKNET':
            network = new StacksMocknet({
                url: `https://${getEnv('MOCKNET_ADDRESS')}`,
            });
            deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
            api_base_extended = `https://${getEnv('MOCKNET_ADDRESS')}/extended/v1`;
            break;
        case 'STACKS_LOCAL':
            network = new StacksMocknet();
            deployer = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
            api_base_extended = 'http://localhost:3999/extended/v1';
            break;
        default:
            throw new Error(`${config.chain} is not a valid chain.`);
    }

    return {
        setStatusFunded: async (uuid) => {
            try {
                const cbPrincipal = await getCallbackContract(uuid, contractName, deployer, network);

                const txOptions2: SignedContractCallOptions = {
                    contractAddress: deployer,
                    contractName: contractName,
                    functionName: 'set-status-funded',
                    functionArgs: [
                        uuidToCV(uuid),
                        contractPrincipalCV(addressToString(cbPrincipal.address), cbPrincipal.contractName.content),
                    ],
                    senderKey: adminKey,
                    validateWithAbi: true,
                    network: network,
                    fee: 100000,
                    anchorMode: 1,
                };

                const transaction2 = await makeContractCall(txOptions2);
                console.log('Transaction payload:', transaction2.payload);
                const broadcastResponse: TxBroadcastResult = await broadcastTransaction(transaction2, network);
                console.log('Broadcast response: ', broadcastResponse);
                return broadcastResponse as any;
            } catch (error) {
                console.log(error);
                return error;
            }
        },

        postCloseDLC: async (uuid, btcTxId) => {
            try {
                const callbackContractPrincipal = await getCallbackContract(uuid, contractName, deployer, network);

                const functionName = 'post-close-dlc';
                const contractNonFungiblePostCondition = makeContractNonFungiblePostCondition(
                    deployer,
                    contractName,
                    NonFungibleConditionCode.Sends,
                    createAssetInfo(deployer, contractName, dlcNFTName),
                    bufferCV(hexToBytes(uuid))
                );

                function populateTxOptions() {
                    return {
                        contractAddress: deployer,
                        contractName: contractName,
                        functionName: functionName,
                        functionArgs: [
                            bufferCV(hexToBytes(uuid)),
                            stringAsciiCV(btcTxId),
                            contractPrincipalCV(
                                addressToString(callbackContractPrincipal.address),
                                callbackContractPrincipal.contractName.content
                            ),
                        ],
                        postConditions: [contractNonFungiblePostCondition],
                        senderKey: adminKey,
                        validateWithAbi: true,
                        network: network,
                        fee: 100000, //0.1STX
                        anchorMode: 1,
                    };
                }

                const transaction = await makeContractCall(populateTxOptions());
                console.log('Transaction payload:', transaction.payload);
                const broadcastResponse = await broadcastTransaction(transaction, network);
                console.log('Broadcast response: ', broadcastResponse);
                return broadcastResponse as any;
            } catch (error) {
                console.log(error);
                return error;
            }
        },

        getAllAttestors: async () => {
            try {
                const data = await getAllAttestors(deployer, contractName, attestorNFTName, api_base_extended);
                const attestors: string[] = data.results.map((res) => res.value.repr);
                return attestors as any;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
    };
};
