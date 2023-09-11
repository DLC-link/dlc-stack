import dotenv from 'dotenv';
dotenv.config();
export function getEnv(key) {
    const value = process.env[key];
    if (!value)
        throw new Error(`Environment variable ${key} is missing.`);
    return value;
}
export default () => {
    let chain = process.env.CHAIN;
    let version = process.env.VERSION;
    let privateKey = process.env.PRIVATE_KEY;
    let apiKey = process.env.API_KEY;
    let branch = process.env.BRANCH || 'master';
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
