import * as http from 'http';
import express from 'express';
import routes from './routes.js';
import ConfigService from '../../services/config.service.js';

export default () => {
    const app = express();
    app.use(routes);

    const server = http.createServer(app);

    const port = ConfigService.getSettings()['private-server-port'] || 3000;

    server.listen(port, () => {
        console.log(`Private WBI Api listening on port ${port}`);
    });

    return server;
};
