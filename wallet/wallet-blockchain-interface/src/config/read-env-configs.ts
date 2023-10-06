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
    console.log(chain);
    let version = process.env.VERSION as string;
    console.log(version)
    let privateKey = process.env.PRIVATE_KEY as string;
    console.log(privateKey)
    let apiKey = process.env.API_KEY as string;
    console.log(apiKey)
    let branch = (process.env.BRANCH as string) || 'master';
    console.log(branch)

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
