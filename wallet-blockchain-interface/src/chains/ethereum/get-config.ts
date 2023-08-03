import { ConfigSet } from '../../config/models.js';
import fetch from 'cross-fetch';
import { ethers } from 'ethers';
import { DeploymentInfo } from '../shared/models/deployment-info.interface.js';
import fs from 'fs';
import { WrappedContract } from '../shared/models/wrapped-contract.interface.js';

async function fetchDeploymentInfo(subchain: string, version: string, branch: string): Promise<DeploymentInfo> {
    const contract = 'DlcManager';
    try {
        const response = await fetch(
            `https://raw.githubusercontent.com/DLC-link/dlc-solidity/${branch}/deploymentFiles/${subchain}/v${version}/${contract}.json`
        );
        return await response.json();
    } catch (error) {
        throw new Error(`Could not fetch deployment info for ${contract} on ${subchain}`);
    }
}

async function getLocalDeploymentInfo(path: string, contract: string, version: string): Promise<DeploymentInfo> {
    try {
        let dp = JSON.parse(fs.readFileSync(`${path}/v${version}/${contract}.json`, 'utf-8'));
        return dp;
    } catch (error) {
        console.log(error);
        throw new Error(`Could not fetch deployment info for ${contract} on local`);
    }
}

export default async (config: ConfigSet): Promise<WrappedContract> => {
    let deploymentInfo: DeploymentInfo = {} as DeploymentInfo;
    let provider: ethers.providers.JsonRpcProvider;
    let wallet: ethers.Wallet;

    switch (config.chain) {
        case 'ETH_MAINNET':
            deploymentInfo = await fetchDeploymentInfo('mainnet', config.version, config.branch);
            provider = new ethers.providers.JsonRpcProvider(`https://mainnet.infura.io/v3/${config.apiKey}`);
            wallet = new ethers.Wallet(config.privateKey, provider);
            break;
        case 'ETH_SEPOLIA':
            deploymentInfo = await fetchDeploymentInfo('sepolia', config.version, config.branch);
            provider = new ethers.providers.JsonRpcProvider(`https://sepolia.infura.io/v3/${config.apiKey}`);
            wallet = new ethers.Wallet(config.privateKey, provider);
            break;
        case 'ETH_GOERLI':
            deploymentInfo = await fetchDeploymentInfo('goerli', config.version, config.branch);
            provider = new ethers.providers.JsonRpcProvider(`https://goerli.infura.io/v3/${config.apiKey}`);
            wallet = new ethers.Wallet(config.privateKey, provider);
            break;
        case 'ETH_LOCAL':
            deploymentInfo = await getLocalDeploymentInfo('./deploymentFiles/localhost', 'DlcManager', config.version); // TODO:
            provider = new ethers.providers.JsonRpcProvider(`http://127.0.0.1:8545`);
            wallet = new ethers.Wallet(config.privateKey, provider);
            break;
        default:
            throw new Error(`Chain ${config.chain} is not supported.`);
            break;
    }

    const contract = new ethers.Contract(
        deploymentInfo.contract.address,
        deploymentInfo.contract.abi,
        provider
    ).connect(wallet);

    return {
        setStatusFunded: async (uuid) => {
            try {
                const gasLimit = await contract.estimateGas.setStatusFunded(uuid);
                const transaction = await contract.setStatusFunded(uuid, {
                    gasLimit: gasLimit.add(10000),
                });
                const txReceipt = await transaction.wait();
                console.log('Funded request transaction receipt: ', txReceipt);
                return txReceipt;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
        postCloseDLC: async (uuid, btcTxId) => {
            try {
                const gasLimit = await contract.estimateGas.postCloseDLC(uuid, btcTxId);
                const transaction = await contract.postCloseDLC(uuid, btcTxId, {
                    gasLimit: gasLimit.add(10000),
                });
                const txReceipt = await transaction.wait();
                console.log('PostCloseDLC transaction receipt: ', txReceipt);
                return txReceipt;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
        getAllAttestors: async () => {
            try {
                const attestors = await contract.getAllAttestors();
                return attestors;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
    };
};
