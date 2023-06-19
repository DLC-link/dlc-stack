use displaydoc::Display;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Display, Error)]
pub enum AttestorError {
    /// asset pair {0} not recorded
    UnrecordedAssetPairError(String),

    /// datetime RFC3339 parsing error: {0}
    DatetimeParseError(#[from] time::error::Parse),

    /// oracle event with uuid {0} not found
    OracleEventNotFoundError(String),

    /// oracle specific database error: {0}
    OracleDatabaseError(String),

    /// storage api error: {0}
    StorageApiError(#[from] dlc_clients::ApiError),
}

// impl actix_web::error::ResponseError for AttestorError {
//     fn status_code(&self) -> actix_web::http::StatusCode {
//         if let AttestorError::DatetimeParseError(_) = self {
//             return actix_web::http::StatusCode::BAD_REQUEST;
//         }
//         actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
//     }
// }
