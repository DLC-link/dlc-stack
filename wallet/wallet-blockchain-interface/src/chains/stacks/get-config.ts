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
} from '@stacks/transactions';
import type { TxBroadcastResult } from '@stacks/transactions';
import { ChainConfig } from '../../config/models.js';
import { WrappedContract } from '../shared/models/wrapped-contract.interface.js';
import { hexToBytes, uuidToCV } from './helper-functions.js';
import { StacksNetwork } from '@stacks/network';
import getNetworkInfo from './get-network-config.js';
import StacksNonceService from '../../services/stacks-nonce.service.js';
import { BigNumber } from 'ethers';

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

export default async (config: ChainConfig): Promise<WrappedContract> => {
    console.log(`[Stacks] Loading contract config for ${config.network}...`);
    const walletKey = config.private_key;
    const contractName = 'dlc-manager-v1-1';

    const { stacksNetwork, deployer, walletAddress } = await getNetworkInfo(config);

    return {
        setStatusFunded: async (uuid, btcTxId) => {
            try {
                const cbPrincipal = await getCallbackContract(uuid, contractName, deployer, stacksNetwork);

                const txOptions2: SignedContractCallOptions = {
                    contractAddress: deployer,
                    contractName: contractName,
                    functionName: 'set-status-funded',
                    functionArgs: [
                        uuidToCV(uuid),
                        stringAsciiCV(btcTxId),
                        contractPrincipalCV(addressToString(cbPrincipal.address), cbPrincipal.contractName.content),
                    ],
                    senderKey: walletKey,
                    validateWithAbi: true,
                    network: stacksNetwork,
                    anchorMode: 1,
                    nonce: await StacksNonceService.getNonce(stacksNetwork, walletAddress),
                };

                const transaction2 = await makeContractCall(txOptions2);
                console.log('Transaction payload:', transaction2.payload);
                const broadcastResponse: TxBroadcastResult = await broadcastTransaction(transaction2, stacksNetwork);
                console.log('Broadcast response: ', broadcastResponse);
                return broadcastResponse as any;
            } catch (error) {
                console.log(error);
                return error;
            }
        },

        postCloseDLC: async (uuid, btcTxId) => {
            try {
                const callbackContractPrincipal = await getCallbackContract(
                    uuid,
                    contractName,
                    deployer,
                    stacksNetwork
                );
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
                        network: stacksNetwork,
                        anchorMode: 1,
                        nonce: await StacksNonceService.getNonce(stacksNetwork, walletAddress),
                    };
                }

                const transaction = await makeContractCall(await populateTxOptions());
                console.log('Transaction payload:', transaction.payload);
                const broadcastResponse = await broadcastTransaction(transaction, stacksNetwork);
                console.log('Broadcast response: ', broadcastResponse);
                return broadcastResponse as any;
            } catch (error) {
                console.log(error);
                return error;
            }
        },

        getDLCInfo: async (uuid) => {
            try {
                console.log('Getting DLC info...');
                const functionName = 'get-dlc';
                const txOptions = {
                    contractAddress: deployer,
                    contractName: contractName,
                    functionName: functionName,
                    functionArgs: [uuidToCV(uuid)],
                    senderAddress: deployer,
                    network: stacksNetwork,
                };
                const transaction: any = await callReadOnlyFunction(txOptions);
                const dlcInfo = cvToValue(transaction.value);
                dlcInfo.refundDelay = BigNumber.from(parseInt(dlcInfo['refund-delay'].value));
                dlcInfo.valueLocked = BigNumber.from(parseInt(dlcInfo['value-locked'].value));
                dlcInfo.btcFeeRecipient = dlcInfo['btc-fee-recipient'].value;
                dlcInfo.btcFeeBasisPoints = BigNumber.from(parseInt(dlcInfo['btc-fee-basis-points'].value));
                return dlcInfo;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
    };
};
