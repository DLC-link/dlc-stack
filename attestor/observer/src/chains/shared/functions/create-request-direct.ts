import fetch from 'cross-fetch';
import strftime from 'strftime';

import { addNewDLC as addNewStacksDLC } from '../../stacks/functions/add-new-dlc';
import { AddStacks } from '../../stacks/models/DTOs/add-stacks.dto';
import { addNewDLCETH as addNewETHDLC } from '../../ethereum/functions/add-new-dlc';
import { AddEthereum } from '../../ethereum/models/DTOs/add-eth.dto';
import { CreateRequestDirectDTO } from '../models/DTOs/create-request-direct.dto';

export async function createRequestDirect(data: CreateRequestDirectDTO) {
  // NOTE: FIXME: TODO:
  const maturationShift = parseInt(process.env.MATURATION_SHIFT as string) || 0;
  let maturation = new Date(new Date().getTime() - maturationShift);

  try {
    const oracleURL = process.env.ORACLE_URL;
    const formattedMaturation = strftime('%Y-%m-%dT%H:%M:%SZ', maturation);
    const URL = `${oracleURL}/v1/create_event/${data.uuid}?maturation=${formattedMaturation}`;
    console.log(`\n************************* Running DLC Oracle Command *************************`);
    console.log(`${URL}`);
    const response = await fetch(URL);
    const responseData = await response.json();
    console.log('Oracle Response:', responseData);
  } catch (error: any) {
    throw new Error('Failed to get Oracle Announcement');
  }

  switch (data.chain) {
    case 'ETHEREUM': {
      return addNewETHDLC({
        uuid: data.uuid,
        creator: data.creator,
        receiver: data.receiver,
        emergencyRefundTime: maturation.getTime(),
        nonce: parseInt(data.nonce),
        sourceContract: data.sourceContract,
      } as AddEthereum);
      break;
    }
    case 'STACKS': {
      return addNewStacksDLC({
        uuid: data.uuid,
        emergencyRefundTime: (maturation.getTime() / 1000).toString(),
        creator: data.creator,
        callbackContract: data.callbackContract,
        nonce: parseInt(data.nonce),
        sourceContract: data.sourceContract,
      } as AddStacks);
      break;
    }
  }
}
