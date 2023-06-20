export interface OracleResponse {
  event_id: string;
  uuid: string;
  suredbits_announcement: string;
  rust_announcement_json: string;
  rust_announcement: string;
  suredbits_attestation: string;
  rust_attestation_json: string;
  rust_attestation: string;
  maturation: string;
  outcome: number;
}
