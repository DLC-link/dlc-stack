import { FunctionName } from '../interfaces/function-names.type';

export interface CloseInternalCallbackDTO {
  uuid: string;
  callbackContract: string;
  assetInfo: {
    contractAddress: string;
    contractName: string;
    NFTName: string;
  };
  functionName: FunctionName;
}
