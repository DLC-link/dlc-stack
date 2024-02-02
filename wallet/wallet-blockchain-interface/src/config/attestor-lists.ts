async function fetchRemoteAttestorList(network: string): Promise<string[]> {
    const branch = process.env.ATTESTOR_LIST_FETCH_BRANCH || 'main';
    try {
        const response = await fetch(
            `https://raw.githubusercontent.com/DLC-link/configurations/${branch}/attestors/attestors.${network}.json`
        );
        const data = await response.json();
        return data;
    } catch (error) {
        console.error('Error fetching remote attestor list:', error);
        return [];
    }
}

const attestorLists: Array<{ name: string; domains: () => Promise<string[]> }> = [
    {
        name: 'docker',
        domains: async () => [
            'http://172.20.128.5:8801', // Docker hardcoded attestors
            'http://172.20.128.6:8802',
            'http://172.20.128.7:8803',
        ],
    },
    {
        name: 'local',
        domains: async () => [
            'http://127.0.0.1:8801', // Local and Just mode
            'http://127.0.0.1:8802',
            'http://127.0.0.1:8803',
        ],
    },
    {
        name: 'devnet',
        domains: async () => fetchRemoteAttestorList('devnet'),
    },
    {
        name: 'testnet',
        domains: async () => fetchRemoteAttestorList('testnet'),
    },
    {
        name: 'mainnet',
        domains: async () => fetchRemoteAttestorList('mainnet'),
    },
];

async function getAttestorList(config: string): Promise<string[]> {
    const list = attestorLists.find((item) => item.name === config);
    return (await list?.domains()) || [];
}

export async function getAttestors(): Promise<string[]> {
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
