import { Attestor } from 'attestor';
import strftime from 'strftime';

// We used to have a maturation shift for testing purposes:
// let maturation = new Date(new Date().getTime() - maturationShift);

export async function createAnnouncement(uuid: string, maturation?: string) {
  const attestor = await Attestor.new();

  let _maturation = maturation ? new Date(maturation) : new Date(new Date().getTime());
  const _formattedMaturation = strftime('%Y-%m-%dT%H:%M:%SZ', _maturation);

  await attestor.create_event(uuid, _formattedMaturation);
}
