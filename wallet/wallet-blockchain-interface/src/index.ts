import privateServer from './http/private-server/server.js';
import publicServer from './http/public-server/server.js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import ConfigService from './services/config.service.js';

async function main() {
    const __filename = fileURLToPath(import.meta.url);
    const __dirname = path.dirname(__filename);
    let options;

    const TLS_ENABLED = ConfigService.getSettings()['tls-enabled'] ?? false;

    if (TLS_ENABLED) {
        options = {
            key: fs.readFileSync(path.resolve(__dirname, '../.cert/server.key')),
            cert: fs.readFileSync(path.resolve(__dirname, '../.cert/server.crt')),
        };
    }

    // Start servers
    publicServer(TLS_ENABLED, options);
    privateServer();
}

main();
