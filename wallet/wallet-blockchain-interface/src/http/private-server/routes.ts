import express from 'express';
import BlockchainWriterService from '../../services/blockchain-writer.service.js';
import { localhostOrDockerOnly } from '../middlewares.js';

const blockchainWriter = await BlockchainWriterService.getBlockchainWriter();
const router = express.Router();
const TESTMODE: boolean = process.env.TEST_MODE_ENABLED === 'true';
const JUSTMODE: boolean = process.env.JUST_MODE === 'true';

router.post('/set-status-funded', express.json(), localhostOrDockerOnly, async (req, res) => {
    console.log('POST /set-status-funded with UUID:', req.body.uuid);
    if (!req.body.uuid) {
        res.status(400).send('Missing UUID');
        return;
    }
    if (TESTMODE || JUSTMODE) {
        res.status(200).send('set-status-funded called in test mode.');
        return;
    }
    const data = await blockchainWriter.setStatusFunded(req.body.uuid as string);
    res.status(200).send(data);
});

router.get('/get-all-attestors', express.json(), localhostOrDockerOnly, async (req, res) => {
    console.log('GET /get-all-attestors');
    let data;
    if (TESTMODE) {
        data = ['http://172.20.128.5:8801', 'http://172.20.128.6:8802', 'http://172.20.128.7:8803'];
    } else if (JUSTMODE) {
        data = ['http://127.0.0.1:8801', 'http://127.0.0.1:8802', 'http://127.0.0.1:8803'];
    } else {
        data = await blockchainWriter.getAllAttestors();
    }
    res.status(200).send(data);
});

router.post('/post-close-dlc', express.json(), localhostOrDockerOnly, async (req, res) => {
    if (!req.body.uuid) {
        res.status(400).send('Missing UUID');
        return;
    }
    if (!req.body.btcTxId) {
        res.status(400).send('Missing BTC TX ID');
        return;
    }
    const { uuid, btcTxId } = req.body;
    console.log('POST /post-close-dlc with UUID, BTC TX ID:', uuid, btcTxId);

    if (TESTMODE || JUSTMODE) {
        res.status(200).send('post-close-dlc called in test mode.');
        return;
    }
    const data = await blockchainWriter.postCloseDLC(uuid as string, btcTxId as string);
    res.status(200).send(data);
});

export default router;
