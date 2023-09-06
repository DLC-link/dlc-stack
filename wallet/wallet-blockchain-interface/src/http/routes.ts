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

router.get('/get-all-attestors', express.json(), async (req, res) => {
    console.log('GET /get-all-attestors');
    let data;
    if (process.env.TEST_MODE_ENABLED === 'true') {
        data = ['http://attestor-1:8801', 'http://attestor-2:8802', 'http://attestor-3:8803'];
    } else {
        data = await blockchainWriter.getAllAttestors();
    }
    res.status(200).send(data);
});

router.post('/post-close-dlc', express.json(), async (req, res) => {
    if (!req.body.uuid) {
        res.status(400).send('Missing UUID');
        return;
    }
    if (!req.body.btcTxId) {
        res.status(400).send('Missing BTC TX ID');
        return;
    }
    const { uuid, btcTxId } = req.body;
    console.log('POST /post-close-dlc with UUID:', uuid);
    const data = await blockchainWriter.postCloseDLC(uuid as string, btcTxId as string);
    res.status(200).send(data);
});

export default router;
