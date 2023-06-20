export type AddStacksDirectDTO = {
  uuid: string;
  emergencyRefundTime?: string;
  creator: string;
  callbackContract: string;
  nonce: number;
  sourceContract: string;
  // NOTE:
  strikePrice?: number;
  currencySymbol?: string;
};

export type AddStacks = AddStacksDirectDTO;
