export interface PostCloseOutcomeDTO {
  uuid: string;
  callbackContract?: string;
  outcome: number;
  assetInfo?: {
    contractAddress: string;
    contractName: string;
    NFTName: string;
  };
}
