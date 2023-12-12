use displaydoc::Display;
use dlc_clients::ApiError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, OracleError>;

#[derive(Clone, Debug, Display, Error)]
pub enum OracleError {
    /// storage api error: {0}
    StorageApiError(#[from] ApiError),
    /// base64 decode error: {0}
    Base64DecodeError(#[from] base64::DecodeError),
}
