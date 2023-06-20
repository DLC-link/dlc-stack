import { ethers } from 'ethers';
// import loggerSvc from '../../../../services/logger.service';
import DLCInfoService from '../../../../services/dlc-info.service';
import { AddEthManagerDTO } from '../../models/DTOs/add-eth.dto';
import { callPostCloseOutcome } from '../../../shared/functions/post-close-outcome';
import { createRequestDirect } from '../../../shared/functions/create-request-direct';
import { mintNft } from '../../functions/mint-nft';
import { postMintNft } from '../../functions/post-mint-nft';
import { EthereumContract } from '../../models/classes/contract';
import { config } from '../../../../utilities/config';

const dlcInfoSvc = DLCInfoService.getSvc();

export class DLCManagerV0 extends EthereumContract {
  handleTx() {
    this.contractWithSigner.on(
      'CreateDLC',
      async (
        _uuid: string,
        _creator: string,
        _receiver: string,
        _emergencyRefundTime: ethers.BigNumber,
        _nonce: ethers.BigNumber,
        _eventSource: string
      ) => {
        const currentTime = new Date();
        const emergencyRefundTime = _emergencyRefundTime.toNumber().toString();
        const nonce = _nonce.toNumber().toString() as any;
        const _logMessage = `[${this.name}][${this.subchain}] New DLC Request... @ ${currentTime} \n\t uuid: ${_uuid} | emergencyRefundTime: ${emergencyRefundTime} | creator: ${_creator} \n`;
        // loggerSvc.log(_logMessage);

        if (!config.txHandlingEnabled) return;

        try {
          const response = await createRequestDirect({
            uuid: _uuid,
            emergencyRefundTime: emergencyRefundTime,
            creator: _creator,
            receiver: _receiver,
            callbackContract: undefined,
            nonce: nonce,
            sourceContract: this.address,
            chain: this.chain,
          });
          console.log('Response after direct request:', response);
        } catch (error) {
          console.error(error);
        }
      }
    );

    this.contractWithSigner.on(
      'PostCreateDLC',
      async (
        _uuid: string,
        _creator: string,
        _receiver: string,
        _emergencyRefundTime: ethers.BigNumber,
        _nonce: ethers.BigNumber,
        _eventSource: string
      ) => {
        const currentTime = new Date();
        const emergencyRefundTime = _emergencyRefundTime.toNumber().toString();
        const nonce = _nonce.toNumber().toString() as any;
        const _logMessage = `[${this.name}][${this.subchain}] New DLC Created @ ${currentTime} \n\t uuid: ${_uuid} | emergencyRefundTime: ${emergencyRefundTime} | creator: ${_creator} | receiver: ${_receiver} \n`;
        // loggerSvc.log(_logMessage);

        if (!config.txHandlingEnabled) return;

        // NOTE:
        dlcInfoSvc.addInfo({
          uuid: _uuid,
          contractAddress: this.address,
          chain: this.chain,
          outcome: 0,
        });
      }
    );

    this.contractWithSigner.on('SetStatusFunded', async (_uuid: string, _eventSource: string) => {
      const currentTime = new Date();
      const _logMessage = `[${this.name}][${this.subchain}] Status set to Funded @ ${currentTime} \n\t uuid: ${_uuid} \n`;
      // loggerSvc.log(_logMessage);
    });

    this.contractWithSigner.on(
      'MintBtcNft',
      async (_uuid: string, _creator: string, _receiver: string, _deposit: ethers.BigNumber, _eventSource: string) => {
        const currentTime = new Date();
        const _logMessage = `[${this.name}][${this.subchain}] NFT Minted @ ${currentTime} \n\t uuid: ${_uuid} | creator: ${_creator} | receiver: ${_receiver} | deposit: ${_deposit} \n`;
        // loggerSvc.log(_logMessage);

        if (!config.txHandlingEnabled) return;

        try {
          const newNftId: number = await mintNft(
            {
              uuid: _uuid,
              collateral: _deposit.toNumber(),
              creator: _creator,
              receiver: _receiver,
              nftContract: this.nftContractAddress,
              sourceContract: this.address,
            },
            this.nftContractWithSigner
          );
          console.log('returning from mintNFT call with nftId:' + newNftId);

          const postMintResponse = await postMintNft({
            uuid: _uuid,
            collateral: _deposit.toNumber(),
            creator: _creator,
            receiver: _receiver,
            sourceContract: this.address,
            nftId: newNftId,
          });
          console.log('Response after postNftMint:', postMintResponse);
        } catch (error) {
          console.error(error);
        }
      }
    );

    this.contractWithSigner.on(
      'CloseDLC',
      async (_uuid: string, _outcome: ethers.BigNumber, _creator: string, _eventSource: string) => {
        const currentTime = new Date();
        const outcome = _outcome.toNumber();
        // loggerSvc.log(
        //   `[${this.name}][${this.subchain}] Closing DLC... @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${outcome} \n`
        // );

        if (!config.txHandlingEnabled) return;

        dlcInfoSvc.addInfo({
          uuid: _uuid,
          contractAddress: this.address,
          chain: this.chain,
          outcome: outcome,
        });

        try {
          callPostCloseOutcome(
            {
              uuid: _uuid,
              outcome: outcome,
            },
            2
          );
        } catch (error) {
          console.error(error);
        }
      }
    );

    this.contractWithSigner.on(
      'PostCloseDLC',
      async (_uuid: string, _outcome: ethers.BigNumber, _actualClosingTime: ethers.BigNumber, _eventSource: string) => {
        const currentTime = new Date();
        const outcome = _outcome.toNumber();
        const actualClosingTime = _actualClosingTime.toNumber();
        // loggerSvc.log(
        //   `[${this.name}][${this.subchain}] Closed DLC @ ${currentTime} \n\t uuid: ${_uuid} | outcome: ${outcome} | actualClosingTime: ${actualClosingTime} \n`
        // );

        if (!config.txHandlingEnabled) return;

        // NOTE: is there any todo here?
      }
    );

    this.contractWithSigner.on(
      'BTCPriceFetching',
      async (_uuid: string, _caller: string, _price: ethers.BigNumber, _eventSource: string) => {
        const currentTime = new Date();
        // loggerSvc.log(
        //   `[${
        //     this.name
        //   }] BTC Price Requested @ ${currentTime} \n\t uuid: ${_uuid} | price: ${_price.toNumber()} | caller: ${_caller} \n`
        // );

        if (!config.txHandlingEnabled) return;

        // NOTE: ?
      }
    );
  }
  getAddNewArgs(params: AddEthManagerDTO) {
    return [params.uuid, params.emergencyRefundTime, params.nonce, params.creator, params.receiver];
  }
}
