export type MintEthereumDTO = {
  uuid: string;
  collateral: number;
  creator: string;
  receiver: string;
  sourceContract: string;
};

export type MintEthNFTDTO = {
  nftContract: string;
};

export type MintEthereum = MintEthereumDTO & MintEthNFTDTO;
