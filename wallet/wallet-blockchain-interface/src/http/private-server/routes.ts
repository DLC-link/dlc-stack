import express from 'express';
import BlockchainInterfaceService from '../../services/blockchain-interface.service.js';
import { localhostOrDockerOnly } from '../middlewares.js';
import { getAttestors } from '../../config/attestor-lists.js';
import ConfigService from '../../services/config.service.js';

const blockchainWriter = await BlockchainInterfaceService.getBlockchainWriter();
const router = express.Router();
const TESTMODE: boolean = ConfigService.getEnv('TEST_MODE_ENABLED') == 'true';

router.post('/set-status-funded', express.json(), localhostOrDockerOnly, async (req, res) => {
    if (!req.body.uuid) {
        res.status(400).send('Missing UUID');
        return;
    }
    if (!req.body.btcTxId) {
        res.status(400).send('Missing BTC TX ID');
        return;
    }
    if (!req.body.chain) {
        res.status(400).send('Missing chain');
        return;
    }
    console.log(
        `[WBI] POST /set-status-funded with UUID: ${req.body.uuid} and BTC TX ID: ${req.body.btcTxId} and chain: ${req.body.chain}`
    );
    if (TESTMODE) {
        res.status(200).send('set-status-funded called in test mode.');
        return;
    }

    const data = await blockchainWriter.setStatusFunded(
        req.body.uuid as string,
        req.body.btcTxId as string,
        req.body.chain as string
    );
    res.status(200).send(data);
});

router.get('/get-all-attestors', express.json(), localhostOrDockerOnly, async (req, res) => {
    console.log('[WBI] GET /get-all-attestors');
    let data = getAttestors();
    console.log('AttestorList:', data);
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
    if (!req.body.chain) {
        res.status(400).send('Missing chain');
        return;
    }
    const { uuid, btcTxId } = req.body;
    console.log('[WBI] POST /post-close-dlc with UUID, BTC TX ID:', uuid, btcTxId);

    if (TESTMODE) {
        res.status(200).send('post-close-dlc called in test mode.');
        return;
    }
    const data = await blockchainWriter.postCloseDLC(uuid as string, btcTxId as string, req.body.chain as string);
    res.status(200).send(data);
});

export default router;
