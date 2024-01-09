import { Attestor } from 'attestor';
import { generateMnemonic, mnemonicToSeedSync } from 'bip39';
import { BIP32Factory } from 'bip32';
import * as ecc from 'tiny-secp256k1';
import ConfigService from './config.service.js';
import { PrefixedChain } from '../config/models.js';
import { createAttestorMetricsCounters } from '../config/prom-metrics.models.js';

function getOrGenerateSecretFromConfig(): string {
  let secretKey: string;
  try {
    secretKey = ConfigService.getEnv('ATTESTOR_XPRIV');
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
  maturationDate.setDate(maturationDate.getDate() + 1);
  return maturationDate.toISOString();
}

const attestorMetricsCounter = createAttestorMetricsCounters();
export default class AttestorService {
  private static attestor: Attestor;

  private constructor() {}

  public static async getAttestor(): Promise<Attestor> {
    if (!this.attestor) {
      this.attestor = await Attestor.new(
        ConfigService.getSettings()['storage-api-endpoint'],
        getOrGenerateSecretFromConfig()
      );
      console.log('Attestor created');
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
      attestorMetricsCounter.getHealthSuccessCounter.inc();
      return JSON.stringify({ data: health_response });
    } catch (error) {
      console.error(error);
      attestorMetricsCounter.getHealthErrorCounter.inc();
      return error;
    }
  }

  public static async createAnnouncement(uuid: string, chain: PrefixedChain, maturation?: string) {
    const attestor = await this.getAttestor();

    console.log('createAnnouncement with UUID:', uuid, 'and maturation:', maturation);

    let _maturation = maturation ? new Date(Number(maturation)).toISOString() : createMaturationDate();

    try {
      await attestor.create_event(uuid, _maturation, chain);
      attestorMetricsCounter.createAnnouncementSuccessCounter.inc();
    } catch (error) {
      console.error(error);
      attestorMetricsCounter.createAnnouncementErrorCounter.inc();
      return error;
    }
    return { uuid: uuid, maturation: _maturation };
  }

  public static async createAttestation(uuid: string, value: bigint, precisionShift = 0) {
    const attestor = await this.getAttestor();

    const formatOutcome = (value: number): bigint => BigInt(Math.round(value / 10 ** precisionShift));
    // We can safely assume that the value is not bigger than 2^53 - 1
    const formattedOutcome = formatOutcome(Number(value));

    try {
      await attestor.attest(uuid, formattedOutcome);
      attestorMetricsCounter.createAttestationSuccessCounter.inc();
    } catch (error) {
      console.error(error);
      attestorMetricsCounter.createAttestationErrorCounter.inc();
      return error;
    }

    return { uuid: uuid, outcome: Number(formattedOutcome) };
  }

  public static async getEvent(uuid: string) {
    const attestor = await this.getAttestor();
    try {
      const event = await attestor.get_event(uuid);
      attestorMetricsCounter.getEventSuccessCounter.inc();
      return event;
    } catch (error) {
      console.error(error);
      attestorMetricsCounter.getEventErrorCounter.inc();
      return null;
    }
  }

  public static async getAllEvents() {
    const attestor = await this.getAttestor();
    try {
      const events = await attestor.get_events();
      attestorMetricsCounter.getAllEventsSuccessCounter.inc();
      return events;
    } catch (error) {
      console.error(error);
      attestorMetricsCounter.getAllEventsErrorCounter.inc();
      return null;
    }
  }

  public static async getPublicKey() {
    const attestor = await this.getAttestor();
    try {
      const publicKey = await attestor.get_pubkey();
      attestorMetricsCounter.getPublicKeySuccessCounter.inc();
      return publicKey;
    } catch (error) {
      console.error(error);
      attestorMetricsCounter.getPublicKeyErrorCounter.inc();
      return null;
    }
  }
}
