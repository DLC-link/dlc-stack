import { ChainConfig } from '../../config/models.js';
import fetch from 'cross-fetch';
import { ethers } from 'ethers';
import { DeploymentInfo } from '../shared/models/deployment-info.interface.js';
import fs from 'fs';
import { WrappedContract } from '../shared/models/wrapped-contract.interface.js';
import ConfigService from '../../services/config.service.js';

async function fetchDeploymentInfo(subchain: string, version: string): Promise<DeploymentInfo> {
    const contract = 'DLCManager';
    const branch = ConfigService.getSettings()['solidity-branch'] || 'master';
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

export default async (config: ChainConfig): Promise<WrappedContract> => {
    const deploymentInfo: DeploymentInfo =
        config.network === 'localhost'
            ? await getLocalDeploymentInfo(
                  './wallet-blockchain-interface/deploymentFiles/localhost',
                  'DLCManager',
                  config.version
              )
            : await fetchDeploymentInfo(config.network, config.version);

    const provider: ethers.providers.JsonRpcProvider = new ethers.providers.JsonRpcProvider(
        `${config.endpoint}${config.api_key ?? ''}`
    );
    const wallet: ethers.Wallet = new ethers.Wallet(config.private_key, provider);

    const contract = new ethers.Contract(
        deploymentInfo.contract.address,
        deploymentInfo.contract.abi,
        provider
    ).connect(wallet);

    return {
        setStatusFunded: async (uuid, btcTxId) => {
            try {
                const gasLimit = await contract.estimateGas.setStatusFunded(uuid, btcTxId);
                const transaction = await contract.setStatusFunded(uuid, btcTxId, {
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
        getDLCInfo: async (uuid) => {
            try {
                const dlcInfo = await contract.getDLC(uuid);
                return dlcInfo;
            } catch (error) {
                console.log(error);
                return error;
            }
        },
    };
};
