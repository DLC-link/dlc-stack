import { BlockchainInterface } from '../chains/shared/models/observer.interface.js';
import { PSBTEventInterface } from '../config/psbt.models.js';
import AttestorService from './attestor.service.js';

export default class PeriodicService {
  public static blockchainInterfaces: BlockchainInterface[] = [];

  public static init(bis: BlockchainInterface[]) {
    this.blockchainInterfaces = bis;
  }

  // TODO: backchecking / manual funded setting? (for testing)

  public static async start(frequencyInSeconds: number): Promise<void> {
    console.log('[PeriodicService] Starting periodic checks with frequency', frequencyInSeconds, 'seconds');
    setInterval(async () => {
      console.log('[PeriodicService] Running check');

      // attestorService returns list of events
      // call ssF for correct chains & contracts
      // store in temp mem to avoid repinging

      const events: PSBTEventInterface[] = (await AttestorService.getConfirmedPSBTEvents()) as PSBTEventInterface[];

      console.log(events);

      for (const event of events) {
        const { chain, uuid } = event;
        const relevantBIs = this.blockchainInterfaces.filter((bi) => bi.chainName === chain);
        const bi: BlockchainInterface | undefined = relevantBIs.find((bi) => bi.checkAndGetVault(uuid));
        if (!bi) {
          console.error('No blockchain interface found for UUID on this chain', uuid, chain);
          continue;
        }
        console.log('Setting vault status to funded for', uuid);
        await bi.setVaultStatusFunded(uuid, event.funding_txid);
      }
    }, 1000 * frequencyInSeconds);
  }
}
