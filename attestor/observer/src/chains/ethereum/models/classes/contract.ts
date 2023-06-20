import { ethers } from 'ethers';
import { Chain, ETHSubchain } from '../../../chain-types.interface';
import { fetchDeployment } from '../../utilities/fetch-deployment';
import { AddEthereum } from '../DTOs/add-eth.dto';
import { environments } from '../../config/networks.config';
import { DeploymentInfo } from '../interfaces/deployment-info.interface';
import { WebSocketProvider } from '../../utilities/WebsocketProvider';

export abstract class EthereumContract {
  chain: Chain;
  subchain: ETHSubchain;
  name: string;
  address!: string;
  abi!: ethers.ContractInterface;
  contractWithSigner!: ethers.Contract;

  // Every DLCManager contract needs to have a NFT contract
  nftContractAddress!: string;
  nftContractWithSigner!: ethers.Contract;

  // These are used to initialize the contract with an address and abi if needed
  managerObj?: { address: string; abi: ethers.ContractInterface };
  nftObj?: { address: string; abi: ethers.ContractInterface };

  provider: ethers.providers.WebSocketProvider | ethers.providers.JsonRpcProvider;

  abstract handleTx(): void;
  abstract getAddNewArgs(params: AddEthereum): Array<any>;

  constructor(
    chain: Chain,
    subchain: ETHSubchain,
    name: string,
    managerObj?: { address: string; abi: ethers.ContractInterface },
    nftObj?: { address: string; abi: ethers.ContractInterface }
  ) {
    this.name = name;
    this.chain = chain;
    this.subchain = subchain;
    this.provider =
      environments[subchain].providerType == 'ws'
        ? new WebSocketProvider(environments[subchain].providerURL)
        : new ethers.providers.JsonRpcProvider(environments[subchain].providerURL);
    if (managerObj) this.managerObj = managerObj;
    if (nftObj) this.nftObj = nftObj;
  }

  async init(isLocal?: boolean) {
    const _wallet = new ethers.Wallet(environments[this.subchain].admin_private_key, this.provider);
    let deploymentInfo: DeploymentInfo;
    let deploymentInfoNFT: DeploymentInfo;

    // If the contract is being initialized with an address and abi, we don't need to fetch the deployment info
    if (this.managerObj && this.nftObj) {
      deploymentInfo = {
        network: this.subchain,
        contract: {
          name: this.name,
          signerAddress: _wallet.address,
          address: this.managerObj.address,
          abi: this.managerObj.abi,
        },
      };
      this.address = this.managerObj.address;
      deploymentInfoNFT = {
        network: this.subchain,
        contract: {
          name: 'BtcNft',
          signerAddress: _wallet.address,
          address: this.nftObj.address,
          abi: this.nftObj.abi,
        },
      };
      this.nftContractAddress = this.nftObj.address;
    } else {
      deploymentInfo = await fetchDeployment(this.name, this.subchain, isLocal);
      this.address = deploymentInfo.contract.address;
      this.abi = deploymentInfo.contract.abi;
      deploymentInfoNFT = await fetchDeployment('BtcNft', this.subchain, isLocal);
      this.nftContractAddress = deploymentInfoNFT.contract.address;
    }

    this.contractWithSigner = this.createContractWithSigner(deploymentInfo, _wallet);
    this.nftContractWithSigner = this.createContractWithSigner(deploymentInfoNFT, _wallet);
  }

  async loadUUIDs(): Promise<Array<string>> {
    // NOTE: ETH Contracts have to implement getAllUUIDs function for the observer to work correctly
    if (!this.contractWithSigner) return [''];
    if (!this.contractWithSigner.getAllUUIDs) return [''];

    return await this.contractWithSigner.getAllUUIDs();
  }

  createContractWithSigner(deploymentInfo: DeploymentInfo, wallet: ethers.Wallet) {
    return new ethers.Contract(deploymentInfo.contract.address, deploymentInfo.contract.abi, this.provider).connect(
      wallet
    );
  }
}
