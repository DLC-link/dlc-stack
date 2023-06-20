import fetch from 'cross-fetch';
import { closeRequestDirectDTO } from '../models/DTOs/close-request-direct.dto';
import { OracleResponse } from '../models/interfaces/oracle-response.interface';

export async function closeRequestDirect(data: closeRequestDirectDTO) {
  let responseData: OracleResponse;
  const oracleURL = process.env.ORACLE_URL;
  const URL = `${oracleURL}/v1/attest/${data.uuid}?outcome=${data.outcome}`;

  console.log(`\n************************* Running DLC Oracle Command *************************`);
  console.log(`${URL}`);

  try {
    const response = await fetch(URL);
    responseData = (await response.json()) as OracleResponse;
    console.log('Oracle Response:', responseData);
    return responseData;
  } catch (error: any) {
    console.error(error);
  }
}
