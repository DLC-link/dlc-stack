import express from 'express';
import RouterWalletService from '../../services/router-wallet.service.js';
import BlockchainInterfaceService from '../../services/blockchain-interface.service.js';
import { BigNumber } from 'ethers';
import ConfigService from '../../services/config.service.js';

const router = express.Router();
const routerWallet = await RouterWalletService.getRouterWallet();
const blockchainWriter = await BlockchainInterfaceService.getBlockchainWriter();
const TESTMODE: boolean = ConfigService.getEnv('TEST_MODE_ENABLED') == 'true';

router.get('/health', express.json(), async (req, res) => {
    const data = ConfigService.getSettings();
    res.status(200).send({ data });
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
    let btcFeeRecipient: string;
    let btcFeeBasisPoints: BigNumber;

    let offerRequest: {
        uuid: string;
        acceptCollateral: number;
        offerCollateral: number;
        totalOutcomes: number;
        refundDelay: number;
        btcFeeRecipient: string;
        btcFeeBasisPoints: number;
    };

    if (TESTMODE) {
        console.log('[WBI] Test mode enabled. Using default collateral.');
        valueLocked = BigNumber.from(req.body.acceptCollateral);
        refundDelay = BigNumber.from(req.body.refundDelay);
        btcFeeRecipient = req.body.btcFeeRecipient;
        btcFeeBasisPoints = BigNumber.from(req.body.btcFeeBasisPoints);
    } else {
        try {
            const dlcInfo = await blockchainWriter.getDLCInfo(uuid);
            console.log('[WBI] DLC Info:', dlcInfo);
            valueLocked = dlcInfo.valueLocked as BigNumber;
            refundDelay = dlcInfo.refundDelay as BigNumber;
            btcFeeRecipient = dlcInfo.btcFeeRecipient as string;
            btcFeeBasisPoints = dlcInfo.btcFeeBasisPoints as BigNumber;
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
        btcFeeRecipient,
        btcFeeBasisPoints: btcFeeBasisPoints.toNumber(),
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
