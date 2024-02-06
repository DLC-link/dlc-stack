import AttestorService from './attestor.service.js';

export default class PeriodicService {
  public static async start(frequencyInSeconds: number): Promise<void> {
    console.log('[PeriodicService] Starting periodic checks with frequency', frequencyInSeconds, 'seconds');
    setInterval(async () => {
      console.log('[PeriodicService] Running check');

      // fetch all vaults
      // filter to created
      // check all of them against attestor service
      // call ssF for ones that are true
      // store in temp mem to avoid repinging
      // skip false ones

      const valid = await AttestorService.checkEvent(uuid);
      if (!valid) return;
    }, 1000 * frequencyInSeconds);
  }
}
