import express from 'express';
import readEnvConfigs from '../../config/read-env-configs.js';
import RouterWalletService from '../../services/router-wallet.service.js';
import BlockchainInterfaceService from '../../services/blockchain-interface.service.js';
import { BigNumber } from 'ethers';
import { getAttestors } from '../../config/attestor-lists.js';

const router = express.Router();
const routerWallet = await RouterWalletService.getRouterWallet();
const blockchainWriter = await BlockchainInterfaceService.getBlockchainWriter();
const TESTMODE: boolean = process.env.TEST_MODE_ENABLED === 'true';

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
    console.log('[WBI] POST /offer');
    const { uuid } = req.body;
    if (!uuid) {
        res.status(400).send('Missing UUID');
        return;
    }

    let valueLocked: BigNumber;
    let refundDelay: BigNumber;
    let offerRequest: {
        uuid: string;
        acceptCollateral: number;
        offerCollateral: number;
        totalOutcomes: number;
        refundDelay: number;
    };

    if (TESTMODE) {
        console.log('[WBI] Test mode enabled. Using default collateral.');
        valueLocked = BigNumber.from(req.body.acceptCollateral);
        refundDelay = BigNumber.from(req.body.refundDelay);
    } else {
        try {
            const dlcInfo = await blockchainWriter.getDLCInfo(uuid);
            console.log('[WBI] DLC Info:', dlcInfo);
            valueLocked = dlcInfo.valueLocked as BigNumber;
            refundDelay = dlcInfo.refundDelay as BigNumber;
        } catch (error) {
            console.log(error);
            res.status(500).send(error);
            return;
        }
    }

    offerRequest = {
        uuid,
        acceptCollateral: valueLocked.toNumber(),
        offerCollateral: 0,
        totalOutcomes: 100,
        refundDelay: refundDelay.toNumber(),
    };

    console.log('[WBI] Offer Request:', offerRequest);

    const data = await routerWallet.getOffer(offerRequest);
    res.status(data.status).send(await data.json());
});

router.put('/offer/accept', express.json(), async (req, res) => {
    const data = await routerWallet.acceptOffer(req.body);
    res.status(data.status).send(await data.json());
});

export default router;
