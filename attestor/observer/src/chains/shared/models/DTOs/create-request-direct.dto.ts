import { Chain } from '../../../chain-types.interface';

export interface CreateRequestDirectDTO {
  uuid: string;
  emergencyRefundTime: string;
  creator: string;
  receiver: string;
  callbackContract?: string;
  nonce: string;
  sourceContract: string;
  chain: Chain;
}
