import express from 'express';
import readEnvConfigs from '../../config/read-env-configs.js';
import RouterWalletService from '../../services/router-wallet.service.js';

const router = express.Router();
const routerWallet = await RouterWalletService.getRouterWallet();

router.get('/health', express.json(), async (req, res) => {
    const data = readEnvConfigs();
    res.status(200).send({ chain: data.chain, version: data.version });
});

router.get('/wallet-health', express.json(), async (req, res) => {
    const data = await routerWallet.getHealth();
    res.status(data.status).send(await data.json());
});

router.get('/info', express.json(), async (req, res) => {
    const data = await routerWallet.getInfo();
    res.status(data.status).send(await data.json());
});

router.post('/offer', express.json(), async (req, res) => {
    const data = await routerWallet.getOffer(req.body);
    res.status(data.status).send(await data.json());
});

router.put('/offer/accept', express.json(), async (req, res) => {
    const data = await routerWallet.acceptOffer(req.body);
    res.status(data.status).send(await data.json());
});

export default router;
