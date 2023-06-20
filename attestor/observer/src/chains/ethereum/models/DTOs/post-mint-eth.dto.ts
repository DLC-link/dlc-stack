export type PostMintEthereumInputDTO = {
  uuid: string;
  collateral: number;
  creator: string;
  receiver: string;
  sourceContract: string;
};

export type PostMintEthereumOutDTO = {
  uuid: string;
  nftId: number;
};

export type PostMintEthereum = PostMintEthereumInputDTO & PostMintEthereumOutDTO;
