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
    bufferCV,
    stringAsciiCV,
    uintCV,
} from '@stacks/transactions';
import type { TxBroadcastResult } from '@stacks/transactions';
import { ConfigSet } from '../../config/models.js';
import { WrappedContract } from '../shared/models/wrapped-contract.interface.js';
import { callReadOnly, hexToBytes, uuidToCV } from './helper-functions.js';
import { StacksNetwork } from '@stacks/network';
import { getEnv } from '../../config/read-env-configs.js';
import { NFTHoldingsData } from '../shared/models/nft-holdings-data.interface.js';
import getNetworkInfo from './get-network-config.js';
import StacksNonceService from '../../services/stacks-nonce.service.js';

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

async function getRegisteredAttestor(deployer: string, contractName: string, id: number, network: StacksNetwork) {
    const txOptions = {
        contractAddress: deployer,
        contractName: contractName,
        functionName: 'get-registered-attestor',
        functionArgs: [uintCV(id)],
        senderAddress: deployer,
        network,
    };
    return await callReadOnly(txOptions);
}

async function getAllAttestors(
    deployer: string,
    contractName: string,
    nftName: string,
    api_base_extended: string,
    network: StacksNetwork
): Promise<string[]> {
    const getAttestorNFTsURL = `${api_base_extended}/tokens/nft/holdings?asset_identifiers=${deployer}.${contractName}::${nftName}&principal=${deployer}.${contractName}`;
    console.log(`[Stacks] Loading registered attestors from ${getAttestorNFTsURL}...`);

    try {
        const response = await fetch(getAttestorNFTsURL);
        const data = (await response.json()) as NFTHoldingsData;
        const attestorIDs: string[] = data.results.map((res) => res.value.repr);
        console.log(`[Stacks] Loaded registered attestorIDs:`, attestorIDs);
        const attestors = await Promise.all(
            attestorIDs.map(async (id) => {
                const attestor = await getRegisteredAttestor(deployer, contractName, parseInt(id.slice(1)), network);
                return attestor.cvToValue.value.dns.value;
            })
        );
        return attestors;
    } catch (err) {
        console.error(err);
        throw err;
    }
}

export default async (config: ConfigSet): Promise<WrappedContract> => {
    console.log(`[Stacks] Loading contract config for ${config.chain}...`);
    const walletKey = getEnv('PRIVATE_KEY');
    const contractName = 'dlc-manager-v1';
    const attestorNFTName = 'dlc-attestors';

    const { network, deployer, api_base_extended } = await getNetworkInfo(config);

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
                    senderKey: walletKey,
                    validateWithAbi: true,
                    network: network,
                    fee: 100000,
                    anchorMode: 1,
                    nonce: await StacksNonceService.getNonce(),
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
                const functionName = 'post-close';

                async function populateTxOptions() {
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
                        senderKey: walletKey,
                        validateWithAbi: true,
                        network: network,
                        fee: 100000, //0.1STX
                        anchorMode: 1,
                        nonce: await StacksNonceService.getNonce(),
                    };
                }

                const transaction = await makeContractCall(await populateTxOptions());
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
                console.log('Getting all attestors...');
                const data = await getAllAttestors(deployer, contractName, attestorNFTName, api_base_extended, network);
                return data as any;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
    };
};
