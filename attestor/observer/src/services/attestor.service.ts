import { Attestor } from 'attestor';
import { getEnv } from '../config/read-env-configs.js';
import { createECDH } from 'crypto';
import { readFileSync, writeFileSync, existsSync } from 'fs';

function getOrGenerateSecretFromConfig(): string {
  let secretKey: string;
  try {
    secretKey = getEnv('PRIVATE_KEY');
  } catch (error) {
    console.warn('No PRIVATE_KEY env var found, generating secret key');
    const ecdh = createECDH('secp256k1');
    ecdh.generateKeys();
    secretKey = ecdh.getPrivateKey('hex');
  }
  return secretKey;
}

function createMaturationDate() {
  const maturationDate = new Date();
  maturationDate.setMinutes(maturationDate.getMinutes() + 3);
  return maturationDate.toISOString();
}

export default class AttestorService {
  private static attestor: Attestor;

  private constructor() {}

  public static async getAttestor(): Promise<Attestor> {
    if (!this.attestor) {
      this.attestor = await Attestor.new(
        getEnv('STORAGE_API_ENABLED') === 'true',
        getEnv('STORAGE_API_ENDPOINT'),
        getOrGenerateSecretFromConfig()
      );
      console.log('Attestor created');
    }
    console.log('Attestor public key:', await this.attestor.get_pubkey());
    return this.attestor;
  }

  public static async init() {
    await this.getAttestor();
  }

  public static async createAnnouncement(uuid: string, maturation?: string) {
    const attestor = await this.getAttestor();

    let _maturation = maturation ? new Date(maturation).toISOString() : createMaturationDate();

    await attestor.create_event(uuid, _maturation);
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
}
