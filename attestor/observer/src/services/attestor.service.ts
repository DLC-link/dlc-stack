import { Attestor } from 'attestor';
import strftime from 'strftime';

export default class AttestorService {
  private static attestor: Attestor;

  private constructor() {}

  public static async getAttestor(): Promise<Attestor> {
    if (!this.attestor) this.attestor = await Attestor.new();
    return this.attestor;
  }

  public static async createAnnouncement(uuid: string, maturation?: string) {
    const attestor = await this.getAttestor();

    let _maturation = maturation ? new Date(maturation) : new Date(new Date().getTime());
    const _formattedMaturation = strftime('%Y-%m-%dT%H:%M:%SZ', _maturation);

    await attestor.create_event(uuid, _formattedMaturation);
  }

  public static async createAttestation(uuid: string, value: bigint) {
    const attestor = await this.getAttestor();
    await attestor.attest(uuid, value);
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
}
