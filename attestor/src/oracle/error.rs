use displaydoc::Display;
use dlc_clients::ApiError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, OracleError>;

#[derive(Clone, Debug, Display, Error)]
pub enum OracleError {
    // nonpositive announcement time offset: {0}; announcement must happen before attestation
    // InvalidAnnouncementTimeError(time::Duration),
    /// storage api error: {0}
    StorageApiError(#[from] ApiError),
    // event not found in datasource
    // EventNotFoundError,

    // duplicate event found in datasource
    // DuplicateEventFoundError,
}
