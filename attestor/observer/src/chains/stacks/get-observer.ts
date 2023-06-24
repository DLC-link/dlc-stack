import { ConfigSet } from '../../config/models.js';
import { Observer } from '../shared/models/observer.interface.js';

export default async (config: ConfigSet): Promise<Observer> => {
  return { start: () => {} };
};
