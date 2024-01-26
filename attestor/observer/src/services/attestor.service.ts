import { Attestor } from 'attestor';
import { getEnv } from '../config/read-env-configs.js';
import { generateMnemonic, mnemonicToSeedSync } from 'bip39';
import { BIP32Factory } from 'bip32';
import * as ecc from 'tiny-secp256k1';
import { hexToBytes } from '../chains/stacks/utilities/helper-functions.js';
import { StacksTestnet } from '@stacks/network';
import { bufferCV, callReadOnlyFunction, cvToValue } from '@stacks/transactions';

// NOTE: temporary
import fs from 'fs';

function getOrGenerateSecretFromConfig(): string {
  let secretKey: string;
  try {
    secretKey = getEnv('ATTESTOR_XPRIV');
  } catch (error) {
    console.warn('No ATTESTOR_XPRIV extended key env var found, generating xpriv key');
    const mnemonic = generateMnemonic();
    const seed = mnemonicToSeedSync(mnemonic);
    const bip32 = BIP32Factory(ecc);
    const node = bip32.fromSeed(seed);
    secretKey = node.toBase58();
  }
  return secretKey;
}

function createMaturationDate() {
  const maturationDate = new Date();
  maturationDate.setMonth(maturationDate.getMonth() + 3);
  return maturationDate.toISOString();
}

export default class AttestorService {
  private static attestor: Attestor;

  private constructor() {}

  public static async getAttestor(): Promise<Attestor> {
    if (!this.attestor) {
      this.attestor = await Attestor.new(getEnv('STORAGE_API_ENDPOINT'), getOrGenerateSecretFromConfig());
      console.log('Attestor created');
      console.log('Attestor public key:', await this.attestor.get_pubkey());
    }
    return this.attestor;
  }

  public static async init() {
    await this.getAttestor();
  }

  public static async getHealth() {
    try {
      let health_response: any[] = [];
      const health = await Attestor.get_health();
      health.get('data').forEach((element: Iterable<readonly [PropertyKey, any]>) => {
        health_response.push(Object.fromEntries(element));
      });
      return JSON.stringify({ data: health_response });
    } catch (error) {
      console.error(error);
      return error;
    }
  }

  public static async createAnnouncement(uuid: string, maturation?: string) {
    const attestor = await this.getAttestor();

    let _maturation = maturation ? new Date(maturation).toISOString() : createMaturationDate();

    try {
      await attestor.create_event(uuid, _maturation);
    } catch (error) {
      console.error(error);
      return error;
    }
    return { uuid: uuid, maturation: _maturation };
  }

  public static async createAttestation(uuid: string, value: bigint, precisionShift = 0) {
    const attestor = await this.getAttestor();

    const formatOutcome = (value: number): bigint => BigInt(Math.round(value / 10 ** precisionShift));
    // We can safely assume that the value is not bigger than 2^53 - 1
    const formattedOutcome = formatOutcome(Number(value));

    await attestor.attest(uuid, formattedOutcome);
    return { uuid: uuid, outcome: Number(formattedOutcome) };
  }

  public static async getEvent(uuid: string) {
    const attestor = await this.getAttestor();
    try {
      const event = await attestor.get_event(uuid);
      return event;
    } catch (error) {
      console.error(error);
      return null;
    }
  }

  public static async getAllEvents() {
    const attestor = await this.getAttestor();
    try {
      const events = await attestor.get_events();
      return events;
    } catch (error) {
      console.error(error);
      return null;
    }
  }

  public static async getPublicKey() {
    const attestor = await this.getAttestor();
    try {
      const publicKey = await attestor.get_pubkey();
      return publicKey;
    } catch (error) {
      console.error(error);
      return null;
    }
  }

  // An example stuck UUID is: 0xe45e6de55cd022a3fc0a4f0f05c3f22fdd6dc22a822cbcb44dafe5214006a595
  public static async forceCheck(uuid: string) {
    const attestor = await this.getAttestor();

    // we have to get the DLC from chain, parse it's status and outcome and pass it down for rust
    // TODO: for now, only stacks testnet hardcoded here, this will change when we merge it upstream with dev
    async function getDLC(uuid: string) {
      const txOptions = {
        contractAddress: 'ST1JHQ5GPQT249ZWG6V4AWETQW5DYA5RHJB0JSMQ3',
        contractName: 'dlc-manager-v1',
        functionName: 'get-dlc',
        functionArgs: [bufferCV(hexToBytes(uuid))],
        senderAddress: 'ST1JHQ5GPQT249ZWG6V4AWETQW5DYA5RHJB0JSMQ3',
        network: new StacksTestnet(),
      };
      const transaction = await callReadOnlyFunction(txOptions);
      return cvToValue(transaction);
    }

    try {
      console.log('getting DLC from chain...');
      const dlc = await getDLC(uuid);

      if (!dlc || !dlc.value) {
        console.log('DLC not found on chain');
        fs.appendFileSync('nonstx.txt', uuid + '\n');
        return null;
      }

      console.log(dlc.value);
      const status = dlc.value['status']?.value;
      let outcome = dlc.value['outcome']?.value?.value;

      // outcome can be NULL, but we don't want to pass that to rust for now for simplicity
      if (!outcome || outcome == null) outcome = 0n;

      const result = await attestor.force_check(uuid, status, outcome);
      return result;
    } catch (error) {
      console.error(error);
      return error;
    }
  }
}
