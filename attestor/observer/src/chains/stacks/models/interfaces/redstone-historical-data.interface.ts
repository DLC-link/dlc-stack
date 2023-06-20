export interface RedStoneHistoricalData {
  timestamp: number;
  provider: string;
  liteSignature: string;
  prices: Array<{ symbol: string; value: number }>;
  signer: string;
}
