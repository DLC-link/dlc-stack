import type { ContractCallTransaction, TransactionEventSmartContractLog } from '@stacks/stacks-blockchain-api-types';
import { cvToValue, deserializeCV } from '@stacks/transactions';
import { FunctionName, UnwrappedPrintEvent } from '../models/interfaces.js';

export default (tx: ContractCallTransaction, eventSources: string[], dlcManagerName: string) => {
  const printEvent = unwrapPrintEvents(tx, eventSources, dlcManagerName);

  return printEvent
    ? printEvent.map((event) => ({
        printEvent: event,
        eventSource: event ? unwrapEventSource(event['event-source']?.value) : undefined,
      }))
    : [];
};

export function unwrapPrintEvents(
  tx: ContractCallTransaction,
  eventSources: string[],
  dlcManagerName: string
): UnwrappedPrintEvent[] | undefined {
  let unwrappedPrintEvents: UnwrappedPrintEvent[] = [];
  tx.events.forEach((event) => {
    if (event.event_type !== 'smart_contract_log') return;
    if (event.contract_log.contract_id !== dlcManagerName) return;
    const _ev = event as TransactionEventSmartContractLog;
    const _upe = cvToValue(deserializeCV(_ev.contract_log.value.hex));
    const _ues = _upe['event-source']?.value;
    if (eventSources.includes(_ues)) {
      unwrappedPrintEvents.push(_upe);
    }
  });

  return unwrappedPrintEvents;
}

export function unwrapEventSource(es: string): {
  event: FunctionName;
  source: string;
} {
  const esSplit = es.split(':');
  return { event: esSplit[1] as FunctionName, source: esSplit[2] };
}
