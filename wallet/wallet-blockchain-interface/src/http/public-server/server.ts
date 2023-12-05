import * as http from 'http';
import express from 'express';
import routes from './routes.js';
import cors from 'cors';

import * as https from 'https';
import ConfigService from '../../services/config.service.js';

export default (TLS_ENABLED: boolean, options?: { key: Buffer; cert: Buffer }) => {
    if (TLS_ENABLED && !options) {
        throw new Error('TLS enabled but no options provided');
    }

    const app = express();
    app.use(cors());
    app.use(routes);

    const server = TLS_ENABLED ? https.createServer(options!, app) : http.createServer(app);

    const port = ConfigService.getSettings()['public-server-port'] || 3003;

    server.listen(port, () => {
        console.log(`Public WBI API listening on port ${port} ${TLS_ENABLED ? 'with TLS enabled' : ''}`);
    });

    return server;
};
