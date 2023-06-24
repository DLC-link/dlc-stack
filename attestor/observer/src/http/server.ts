import * as http from 'http';
import express from 'express';
import routes from './routes.js';

export default () => {
  const app = express();
  app.use(routes);

  const server = http.createServer(app);
  server.listen(process.env.PORT || 3000, () => {
    console.log('Server listening on port 3000');
  });

  return server;
};
