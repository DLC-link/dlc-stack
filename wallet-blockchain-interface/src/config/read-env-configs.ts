import dotenv from 'dotenv';
import { Chain, ConfigSet, validChains } from './models.js';

dotenv.config();

export function getEnv(key: string): string {
    const value = process.env[key];
    if (!value) throw new Error(`Environment variable ${key} is missing.`);
    return value;
}

export default (): ConfigSet => {
    let chain = process.env.CHAIN as Chain;
    let version = process.env.VERSION as string;
    let privateKey = process.env.PRIVATE_KEY as string;
    let apiKey = process.env.API_KEY as string;
    let branch = (process.env.BRANCH as string) || 'master';

    // Throw an error if one of the set is missing
    if (!chain || !version || !privateKey || !apiKey)
        throw new Error(`CHAIN, VERSION, PRIVATE_KEY, or API_KEY is missing.`);

    return {
        chain: chain,
        version: version,
        privateKey: privateKey,
        apiKey: apiKey,
        branch: branch,
    };
};
