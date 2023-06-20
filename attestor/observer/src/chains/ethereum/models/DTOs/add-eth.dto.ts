export type AddEthManagerDTO = {
  uuid: string;
  creator: string;
  emergencyRefundTime: number;
  nonce: number;
  receiver: string;
  sourceContract: string;
};

export type AddEthereum = AddEthManagerDTO;
