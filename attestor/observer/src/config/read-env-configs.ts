import dotenv from 'dotenv';
import { Chain, ConfigSet, validChains } from './models.js';

dotenv.config();

export default () => {
  let configSets: ConfigSet[] = [];

  for (let i = 1; ; i++) {
    let chain = process.env[`CHAIN_${i}` as keyof NodeJS.ProcessEnv] as Chain;
    let version = process.env[`VERSION_${i}` as keyof NodeJS.ProcessEnv];
    let apiKey = process.env[`API_KEY_${i}` as keyof NodeJS.ProcessEnv];

    // Break the loop if we reach a set of variables that is not defined
    if (!chain && !version && !apiKey) {
      break;
    }

    // Throw an error if one of the set is missing
    if (!chain || !version) {
      throw new Error(`CHAIN_${i} or VERSION_${i} is missing.`);
    }

    // Throw an error if the chain is not one of the predetermined set
    if (!validChains.includes(chain)) {
      throw new Error(`CHAIN_${i}: ${chain} is not a valid chain.\nValid chains are: ${validChains.join(', ')}`);
    }

    configSets.push({
      chain: chain,
      version: version,
      apiKey: apiKey,
    });
  }
  return configSets;
};
