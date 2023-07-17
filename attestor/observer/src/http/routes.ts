import express from 'express';
import AttestorService from '../services/attestor.service.js';

const router = express.Router();

router.get('/health', (req, res) => {
  res.status(200).send('OK');
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

router.get('/public-key', async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  console.log('GET /public-key');
  const data = await AttestorService.getPublicKey();
  res.status(200).send(data);
});

export default router;
