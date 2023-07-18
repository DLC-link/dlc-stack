import { Attestor } from 'attestor';
import { getEnv } from '../config/read-env-configs.js';
import { randomBytes, createECDH } from 'crypto';
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join } from 'path';

async function getOrGenerateSecretFromConfig(secretKeyFile?: string): Promise<string> {
  let secretKeyPath = secretKeyFile || join('config', 'secret.key');
  let secretKey: string;

  if (existsSync(secretKeyPath)) {
    console.log(`Reading secret key from ${secretKeyPath}`);
    secretKey = readFileSync(secretKeyPath, { encoding: 'utf8' }).trim();
  } else {
    console.log('No secret key file was found, generating secret key');
    const ecdh = createECDH('secp256k1');
    ecdh.generateKeys();
    secretKey = ecdh.getPrivateKey('hex');
    writeFileSync(secretKeyPath, secretKey);
  }

  return secretKey;
}

function createMaturationDate() {
  const maturationDate = new Date();
  maturationDate.setMinutes(maturationDate.getMinutes() + 1);
  return maturationDate.toISOString();
}

export default class AttestorService {
  private static attestor: Attestor;

  private constructor() {}

  public static async getAttestor(): Promise<Attestor> {
    if (!this.attestor)
      this.attestor = await Attestor.new(
        getEnv('STORAGE_API_ENABLED') === 'true',
        getEnv('STORAGE_API_ENDPOINT'),
        await getOrGenerateSecretFromConfig('../config/secret.key')
      );
    return this.attestor;
  }

  public static async createAnnouncement(uuid: string, maturation?: string) {
    const attestor = await this.getAttestor();

    let _maturation = maturation ? new Date(maturation).toISOString() : createMaturationDate();

    await attestor.create_event(uuid, _maturation);
  }

  public static async createAttestation(uuid: string, value: bigint, precisionShift = 0) {
    const attestor = await this.getAttestor();

    const formatOutcome = (value: number): bigint => BigInt(Math.round(value / 10 ** precisionShift));
    // We can safely assume that the value is not bigger than 2^53 - 1
    const formattedOutcome = formatOutcome(Number(value));

    await attestor.attest(uuid, formattedOutcome);
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
