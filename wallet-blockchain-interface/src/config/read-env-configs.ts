import dotenv from 'dotenv';
import { Chain, ConfigSet, validChains } from './models.js';

dotenv.config();

export default (): ConfigSet => {
    let chain = process.env.CHAIN as Chain;
    let version = process.env.VERSION as string;
    let privateKey = process.env.PRIVATE_KEY as string;
    let apiKey = process.env.API_KEY as string;

    // Throw an error if one of the set is missing
    if (!chain || !version || !privateKey || !apiKey)
        throw new Error(`CHAIN, VERSION, PRIVATE_KEY, or API_KEY is missing.`);

    return {
        chain: chain,
        version: version,
        privateKey: privateKey,
        apiKey: apiKey,
    };
};
