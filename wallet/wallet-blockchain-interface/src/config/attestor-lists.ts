const attestorLists: Array<{ name: string; domains: string[] }> = [
    {
        name: 'docker',
        domains: [
            'http://172.20.128.5:8801', // Docker hardcoded attestors
            'http://172.20.128.6:8802',
            'http://172.20.128.7:8803',
        ],
    },
    {
        name: 'local',
        domains: [
            'http://127.0.0.1:8801', // Local and Just mode
            'http://127.0.0.1:8802',
            'http://127.0.0.1:8803',
        ],
    },
    {
        name: 'devnet',
        domains: [
            'https://devnet.dlc.link/attestor-1',
            'https://devnet.dlc.link/attestor-2',
            'https://devnet.dlc.link/attestor-3',
        ],
    },
    {
        name: 'testnet',
        domains: [
            'https://testnet.dlc.link/attestor-1',
            'https://testnet.dlc.link/attestor-2',
            'https://testnet.dlc.link/attestor-3',
        ],
    },
    { name: 'mainnet', domains: ['', '', ''] },
];

function getAttestorList(config: string): string[] {
    const list = attestorLists.find((item) => item.name === config);
    return list?.domains || [];
}

export function getAttestors(): string[] {
    // based on two things this will return the attestor list
    // 1. if there is an ATTESTOR_LIST env variable with non-zero length, it will return that list
    // 2. if there is an ATTESTOR_CONFIG env variable, it will return the list for that config

    const attestorList = process.env.ATTESTOR_LIST;
    const attestorConfig = process.env.ATTESTOR_CONFIG;
    if (attestorList && attestorList.length > 0) {
        return attestorList.split(',');
    }
    if (attestorConfig) {
        return getAttestorList(attestorConfig);
    }
    return [];
}
