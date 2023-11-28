import express from 'express';
import dotenv from 'dotenv';
dotenv.config();
import AttestorService from '../services/attestor.service.js';
import ConfigService from '../services/config.service.js';
import chalk from 'chalk';

const router = express.Router();

router.get('/health', async (req, res) => {
  const data = await AttestorService.getHealth();
  res.status(200).send(data);
});

router.get('/event/:uuid', async (req, res) => {
  if (!req.params.uuid) {
    res.status(400).send('Missing UUID');
    return;
  }
  res.setHeader('Access-Control-Allow-Origin', '*');
  console.log('GET /event with UUID:', req.params.uuid);
  const data = await AttestorService.getEvent(req.params.uuid as string);
  res.status(200).send(data);
});

router.get('/events', async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  console.log('GET /events');
  const data = await AttestorService.getAllEvents();
  res.status(200).send(data);
});

router.get('/publickey', async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  console.log('GET /publickey');
  const data = await AttestorService.getPublicKey();
  res.status(200).send(data);
});

if (ConfigService.getSettings()['dev-endpoints-enabled']) {
  console.log(chalk.bgYellow('Dev endpoints enabled!'));
  router.get('/create-announcement/:uuid', async (req, res) => {
    if (!req.params.uuid) {
      res.status(400).send('Missing UUID');
      return;
    }
    res.setHeader('Access-Control-Allow-Origin', '*');
    console.log('GET /create-announcement with UUID:', req.params.uuid);
    const data = await AttestorService.createAnnouncement(req.params.uuid as string);
    res.status(200).send(data);
  });

  router.get('/create-attestation/:uuid/:outcome', async (req, res) => {
    if (!req.params.uuid || !req.params.outcome) {
      res.status(400).send('Missing UUID or outcome');
      return;
    }
    res.setHeader('Access-Control-Allow-Origin', '*');
    console.log('GET /create-attestation with UUID:', req.params.uuid, 'and outcome:', req.params.outcome);
    const data = await AttestorService.createAttestation(req.params.uuid as string, BigInt(req.params.outcome));
    res.status(200).send(data);
  });
}

export default router;
