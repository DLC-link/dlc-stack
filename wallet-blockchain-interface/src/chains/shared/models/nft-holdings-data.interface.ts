export interface Value {
    hex: string;
    repr: string;
}

export interface Result {
    asset_identifier: string;
    value: Value;
    block_height: number;
    tx_id: string;
}
export interface NFTHoldingsData {
    limit: number;
    offset: number;
    total: number;
    results: Result[];
}
