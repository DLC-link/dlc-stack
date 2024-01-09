import { Counter } from 'prom-client';
import { PrefixedChain } from './models.js';

const createAttestorMetricsCounter = (name: string, help: string) =>
  new Counter({ name: `${'attestor_js'}_${name}`, help });

export function createAttestorMetricsCounters() {
  return {
    getHealthSuccessCounter: createAttestorMetricsCounter(
      'get_health_success_counter',
      'Number of health checks requested successfully'
    ),
    getHealthErrorCounter: createAttestorMetricsCounter(
      'get_health_error_counter',
      'Number of errors when requesting health checks'
    ),
    getEventSuccessCounter: createAttestorMetricsCounter(
      'get_event_success_counter',
      'Number of events requested successfully'
    ),
    getEventErrorCounter: createAttestorMetricsCounter(
      'get_event_error_counter',
      'Number of errors when requesting events'
    ),
    getAllEventsSuccessCounter: createAttestorMetricsCounter(
      'get_all_events_success_counter',
      'Number of all events requested successfully'
    ),
    getAllEventsErrorCounter: createAttestorMetricsCounter(
      'get_all_events_error_counter',
      'Number of errors when requesting all events'
    ),
    getPublicKeySuccessCounter: createAttestorMetricsCounter(
      'get_public_key_success_counter',
      'Number of public keys requested successfully'
    ),
    getPublicKeyErrorCounter: createAttestorMetricsCounter(
      'get_public_key_error_counter',
      'Number of errors when requesting public keys'
    ),
    createAnnouncementSuccessCounter: createAttestorMetricsCounter(
      'create_announcement_success_counter',
      'Number of announcements created successfully'
    ),
    createAnnouncementErrorCounter: createAttestorMetricsCounter(
      'create_announcement_error_counter',
      'Number of errors when creating announcements'
    ),
    createAttestationSuccessCounter: createAttestorMetricsCounter(
      'create_attestation_success_counter',
      'Number of attestations created successfully'
    ),
    createAttestationErrorCounter: createAttestorMetricsCounter(
      'create_attestation_error_counter',
      'Number of errors when creating attestations'
    ),
  };
}

const createBlockchainObserverMetricsCounter = (network: PrefixedChain, name: string, help: string, version?: string) => {
  const formattedNetwork = network.replace(/-/g, '_');
  return new Counter({ name: `${'blockchain'}_${formattedNetwork}_${version ?? '1'}_${name}`, help });
};

export interface BlockchainObserverMetricsCounters {
  createDLCEventCounter: Counter<string>;
  setStatusFundedEventCounter: Counter<string>;
  closeDLCEventCounter: Counter<string>;
  postCloseDLCEventCounter: Counter<string>;
}

export function createBlockchainObserverMetricsCounters(network: PrefixedChain, version?: string) {
  return {
    createDLCEventCounter: createBlockchainObserverMetricsCounter(
      network,
      'create_dlc_event_counter',
      'Number of create dlc events received',
      version
    ),
    setStatusFundedEventCounter: createBlockchainObserverMetricsCounter(
      network,
      'set_status_funded_event_counter',
      'Number of set status funded events received',
      version
    ),
    closeDLCEventCounter: createBlockchainObserverMetricsCounter(
      network,
      'close_dlc_event_counter',
      'Number of close dlc events received',
      version
    ),
    postCloseDLCEventCounter: createBlockchainObserverMetricsCounter(
      network,
      'post_close_dlc_event_counter',
      'Number of post close dlc events received',
      version
    ),
  };
}
