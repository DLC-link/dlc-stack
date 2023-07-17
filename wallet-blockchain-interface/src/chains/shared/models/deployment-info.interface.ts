import { ethers } from 'ethers';

export interface DeploymentInfo {
    network: string;
    contract: {
        name: string;
        address: string;
        signerAddress: string;
        abi: string | ethers.ContractInterface;
    };
}
