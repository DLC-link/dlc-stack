enum DLCStatus {
  READY,
  FUNDED,
  CLOSING,
  CLOSED,
}

export interface EVMDLCInterface {
  uuid: string;
  protocolWallet: string;
  protocolContract: string;
  timestamp: number;
  valueLocked: number;
  refundDelay: number;
  creator: string;
  outcome: number;
  status: DLCStatus;
  fundingTxId: string;
  closingTxId: string;
  btcFeeRecipient: string;
  btcFeeBasisPoints: number;
}
