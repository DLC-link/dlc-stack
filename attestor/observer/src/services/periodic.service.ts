import { BlockchainInterface } from '../config/blockchain-interface.interface.js';
import { PSBTEventInterface } from '../config/psbt.models.js';
import AttestorService from './attestor.service.js';

export default class PeriodicService {
  public static blockchainInterfaces: BlockchainInterface[] = [];
  private static uuidLastProcessed: Map<string, number> = new Map();
  private static retryInterval = (parseInt(process.env.RETRY_INTERVAL_SECONDS as string) || 60) * 60 * 1000; // default 60 minutes

  public static init(bis: BlockchainInterface[]) {
    this.blockchainInterfaces = bis;
  }

  public static async start(frequencyInSeconds: number): Promise<void> {
    console.log('[PeriodicService] Starting periodic checks with frequency', frequencyInSeconds, 'seconds');
    setInterval(async () => {
      console.log('[PeriodicService] Running check');

      const events: PSBTEventInterface[] = await AttestorService.getConfirmedPSBTEvents();

      console.log(events);

      const now = Date.now();

      const promises = events.map(async (event) => {
        const { chain_name, uuid } = event;
        const lastProcessed = this.uuidLastProcessed.get(uuid);

        console.log(
          'lastProcessed',
          lastProcessed,
          'now',
          now,
          'retryInterval',
          this.retryInterval,
          'diff',
          lastProcessed && now - lastProcessed,
          'uuid',
          uuid,
          'chain_name',
          chain_name
        );
        if (lastProcessed && now - lastProcessed < this.retryInterval) {
          console.log(`Skipping ${uuid} as it was processed recently.`);
          return;
        }

        const bi = this.blockchainInterfaces.find((bi) => bi.chainName === chain_name && bi.checkAndGetVault(uuid));

        if (!bi) {
          console.error('No blockchain interface found for UUID on this chain', uuid, chain_name);
          return;
        }

        console.log('Setting vault status to funded for:', uuid);
        this.uuidLastProcessed.set(uuid, now);
        await bi.setVaultStatusFunded(uuid, event.funding_txid);
      });

      await Promise.all(promises);
    }, 1000 * frequencyInSeconds);
  }
}
