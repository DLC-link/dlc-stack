import express from 'express';
import BlockchainWriterService from '../services/blockchain-writer.service.js';
import readEnvConfigs from '../config/read-env-configs.js';

const blockchainWriter = await BlockchainWriterService.getBlockchainWriter();
const router = express.Router();

router.get('/health', express.json(), async (req, res) => {
    const data = readEnvConfigs();
    console.log('GET /health', data);
    res.status(200).send({ chain: data.chain, version: data.version });
});

router.post('/set-status-funded', express.json(), async (req, res) => {
    console.log('POST /set-status-funded with UUID:', req.body.uuid);
    if (!req.body.uuid) {
        res.status(400).send('Missing UUID');
        return;
    }
    const data = await blockchainWriter.setStatusFunded(req.body.uuid as string);
    res.status(200).send(data);
});

router.post('/post-close-dlc', express.json(), async (req, res) => {
    if (!req.query.uuid) {
        res.status(400).send('Missing UUID');
        return;
    }
    if (!req.query.btcTxId) {
        res.status(400).send('Missing BTC TX ID');
        return;
    }
    console.log('POST /post-close-dlc with UUID:', req.query.uuid);
    const data = await blockchainWriter.postCloseDLC(req.query.uuid as string, req.query.btcTxId as string);
    res.status(200).send(data);
});

export default router;
