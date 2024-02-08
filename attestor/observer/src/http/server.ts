import * as http from 'http';
import express from 'express';
import routes from './routes.js';
import swStats from 'swagger-stats';
import cors from 'cors';
import bodyParser from 'body-parser';
// const apiSpec = require('swagger.json');

export default () => {
  const app = express();
  const corsOptions = {
    origin: '*',
    credentials: true, //access-control-allow-credentials:true
    optionSuccessStatus: 200,
  };
  app.use(cors(corsOptions)); // Use this after the variable declaration
  // parse application/x-www-form-urlencoded
  app.use(bodyParser.urlencoded({ extended: false }));

  // parse application/json
  app.use(bodyParser.json());
  app.use(swStats.getMiddleware());
  app.use(routes);

  const server = http.createServer(app);

  const port = parseInt(process.env.PORT as string) || 3000;

  server.listen(port, () => {
    console.log(`Server listening on port ${port}`);
  });

  return server;
};
