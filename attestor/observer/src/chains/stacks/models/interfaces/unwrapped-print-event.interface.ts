import { ArgumentName } from './argument-names.type';

export type UnwrappedPrintEvent = {
  [arg in ArgumentName]?: {
    value: any;
  };
};
