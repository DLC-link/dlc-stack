import { ConfigSet } from '../../config/models.js';
import { Observer } from '../shared/models/observer.interface.js';

export default (config: ConfigSet): Observer => {
  return { start: () => {} };
};
