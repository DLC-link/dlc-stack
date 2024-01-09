import { Counter } from 'prom-client';

 const getHealthSuccessCounter = new Counter({
  name: 'get_health_success_counter',
  help: 'Number of health checks requested succesfully',
});

 const getHealthErrorCounter = new Counter({
  name: 'get_health_error_counter',
  help: 'Number of errors when requesting health checks',
});

 const getEventSuccessCounter = new Counter({
  name: 'get_event_success_counter',
  help: 'Number of events requested succesfully',
});

 const getEventErrorCounter = new Counter({
  name: 'get_event_error_counter',
  help: 'Number of errors when requesting events',
});

 const getAllEventsSuccessCounter = new Counter({
  name: 'get_all_events_success_counter',
  help: 'Number of all events requested succesfully',
});

 const getAllEventsErrorCounter = new Counter({
  name: 'get_all_events_error_counter',
  help: 'Number of errors when requesting all events',
});

 const getPublicKeySuccessCounter = new Counter({
  name: 'get_public_key_success_counter',
  help: 'Number of public keys requested succesfully',
});

 const getPublicKeyErrorCounter = new Counter({
  name: 'get_public_key_error_counter',
  help: 'Number of errors when requesting public keys',
});

 const createAnnouncementSuccessCounter = new Counter({
  name: 'create_announcement_success_counter',
  help: 'Number of announcements created succesfully',
});

 const createAnnouncementErrorCounter = new Counter({
  name: 'create_announcement_error_counter',
  help: 'Number of errors when creating announcements',
});

 const createAttestationSuccessCounter = new Counter({
  name: 'create_attestation_success_counter',
  help: 'Number of attestations created succesfully',
});

const createAttestationErrorCounter = new Counter({
  name: 'create_attestation_error_counter',
  help: 'Number of errors when creating attestations',
});

export const metricsCounters = {
    getHealthSuccessCounter,
    getHealthErrorCounter,
    getEventSuccessCounter,
    getEventErrorCounter,
    getAllEventsSuccessCounter,
    getAllEventsErrorCounter,
    getPublicKeySuccessCounter,
    getPublicKeyErrorCounter,
    createAnnouncementSuccessCounter,
    createAnnouncementErrorCounter,
    createAttestationSuccessCounter,
    createAttestationErrorCounter,
}
