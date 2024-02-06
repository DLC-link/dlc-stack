import { BlockchainInterface } from '../chains/shared/models/observer.interface.js';
import AttestorService from './attestor.service.js';

export default class PeriodicService {
  public static blockchainInterfaces: BlockchainInterface[] = [];

  public static init(bis: BlockchainInterface[]) {
    this.blockchainInterfaces = bis;
  }

  public static async start(frequencyInSeconds: number): Promise<void> {
    console.log('[PeriodicService] Starting periodic checks with frequency', frequencyInSeconds, 'seconds');
    setInterval(async () => {
      console.log('[PeriodicService] Running check');

      // attestorService returns list of events
      // call ssF for correct chains & contracts
      // store in temp mem to avoid repinging

      const list = [{ uuid: '123', closing_psbt: { fund: 'ts', close: 'sldksdlk' }, chain: 'evm-sepolia' }];

      for (const event of list) {
        const { chain, uuid } = event;
        const relevantBIs = this.blockchainInterfaces.filter((bi) => bi.chainName === chain);
        const bi: BlockchainInterface | undefined = relevantBIs.find((bi) => bi.checkAndGetVault(uuid));
        if (!bi) {
          console.error('No blockchain interface found for chain', chain);
          continue;
        }
        await bi.setVaultStatusFunded(uuid, event.closing_psbt.fund);
      }
    }, 1000 * frequencyInSeconds);
  }
}
