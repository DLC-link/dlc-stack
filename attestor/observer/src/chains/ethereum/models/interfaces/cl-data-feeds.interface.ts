export interface Proxy {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy[];
}

export interface EthereumAddresses {
  title: string;
  feedType: string;
  networks: Network[];
}

export interface Proxy2 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network2 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy2[];
}

export interface BnbChainAddressesPrice {
  title: string;
  feedType: string;
  networks: Network2[];
}

export interface Proxy3 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network3 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy3[];
}

export interface MaticAddresses {
  title: string;
  feedType: string;
  networks: Network3[];
}

export interface Proxy4 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network4 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy4[];
}

export interface DataFeedsGnosisChain {
  title: string;
  feedType: string;
  networks: Network4[];
}

export interface Proxy5 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network5 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy5[];
}

export interface HuobiEcoChainPriceFeeds {
  title: string;
  feedType: string;
  networks: Network5[];
}

export interface Proxy6 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network6 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy6[];
}

export interface AvalanchePriceFeeds {
  title: string;
  feedType: string;
  networks: Network6[];
}

export interface Proxy7 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network7 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy7[];
}

export interface FantomPriceFeeds {
  title: string;
  feedType: string;
  networks: Network7[];
}

export interface Proxy8 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network8 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy8[];
}

export interface ArbitrumPriceFeeds {
  title: string;
  feedType: string;
  networks: Network8[];
}

export interface Proxy9 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network9 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy9[];
}

export interface HarmonyPriceFeeds {
  title: string;
  feedType: string;
  networks: Network9[];
}

export interface Proxy10 {
  pair: string;
  assetName: string;
  deviationThreshold?: any;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network10 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy10[];
}

export interface DataFeedsSolana {
  title: string;
  feedType: string;
  networks: Network10[];
}

export interface Proxy11 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network11 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy11[];
}

export interface OptimismPriceFeeds {
  title: string;
  feedType: string;
  networks: Network11[];
}

export interface Proxy12 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network12 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy12[];
}

export interface DataFeedsMoonriver {
  title: string;
  feedType: string;
  networks: Network12[];
}

export interface Proxy13 {
  pair: string;
  assetName: string;
  deviationThreshold: number;
  heartbeat: string;
  decimals: number;
  proxy: string;
  feedCategory: string;
  feedType: string;
}

export interface Network13 {
  name: string;
  url: string;
  networkType: string;
  proxies: Proxy13[];
}

export interface DataFeedsMoonbeam {
  title: string;
  feedType: string;
  networks: Network13[];
}

export interface ICLDataFeed {
  'ethereum-addresses': EthereumAddresses;
  'bnb-chain-addresses-price': BnbChainAddressesPrice;
  'matic-addresses': MaticAddresses;
  'data-feeds-gnosis-chain': DataFeedsGnosisChain;
  'huobi-eco-chain-price-feeds': HuobiEcoChainPriceFeeds;
  'avalanche-price-feeds': AvalanchePriceFeeds;
  'fantom-price-feeds': FantomPriceFeeds;
  'arbitrum-price-feeds': ArbitrumPriceFeeds;
  'harmony-price-feeds': HarmonyPriceFeeds;
  'data-feeds-solana': DataFeedsSolana;
  'optimism-price-feeds': OptimismPriceFeeds;
  'data-feeds-moonriver': DataFeedsMoonriver;
  'data-feeds-moonbeam': DataFeedsMoonbeam;
}
