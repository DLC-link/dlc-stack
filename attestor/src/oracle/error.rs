use std::fmt;

use displaydoc::Display;
use dlc_clients::ApiError;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct GenericOracleError {
    pub message: String,
}

impl fmt::Display for GenericOracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GenericOracleError: {}", self.message)
    }
}

impl std::error::Error for GenericOracleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Clone, Debug, Display, Error)]
pub enum OracleError {
    /// storage api error: {0}
    StorageApiError(#[from] ApiError),
    /// base64 decode error: {0}
    Base64DecodeError(#[from] base64::DecodeError),
}
