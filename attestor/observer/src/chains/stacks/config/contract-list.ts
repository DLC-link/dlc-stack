import { ContractConfig } from '../models/interfaces/contract-config.interface';
import { contractConfig as P_v0_1 } from './contract-configs/priced_v0_1.config';

// This is where Stacks dlc-manager contracts have to be registered
export const contracts: ContractConfig[] = [P_v0_1];
