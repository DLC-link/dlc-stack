import { StacksMainnet, StacksMocknet, StacksTestnet } from '@stacks/network';

export interface NetworkConfig {
  network: StacksMainnet | StacksMocknet | StacksTestnet;
  api_base: string;
  api_base_extended: string;
  ioclient_uri: string;
  admin_address: string;
  admin_private_key: string;
}
