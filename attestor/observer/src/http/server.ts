import * as http from 'http';
import express from 'express';
import routes from './routes.js';
import swStats from 'swagger-stats';
// const apiSpec = require('swagger.json');

export default () => {
  const app = express();
  app.use(swStats.getMiddleware());
  app.use(routes);

  const server = http.createServer(app);

  const port = parseInt(process.env.PORT as string) || 3000;

  server.listen(port, () => {
    console.log(`Server listening on port ${port}`);
  });

  return server;
};
