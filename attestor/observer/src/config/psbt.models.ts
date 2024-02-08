interface ClosingPsbtInterface {
  unsigned_tx: object;
  version: number;
  xpub: Map<any, any>;
  proprietary: any[];
  unknown: any[];
  inputs: any[];
  outputs: any[];
}

export interface PSBTEventInterface {
  event_id: string;
  uuid: string;
  funding_txid: string;
  closing_psbt: ClosingPsbtInterface;
  mint_address: string;
  outcome: string | undefined;
  status: string;
  chain_name: string;
}
