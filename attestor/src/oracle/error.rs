use displaydoc::Display;
use dlc_clients::ApiError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, OracleError>;

#[derive(Clone, Debug, Display, Error)]
pub enum OracleError {
    /// storage api error: {0}
    StorageApiError(#[from] ApiError),
}
