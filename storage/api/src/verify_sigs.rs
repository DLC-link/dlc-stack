use std::{
    future::{ready, Ready},
    rc::Rc,
    str::FromStr,
    sync::Mutex,
};

use actix_http::{h1, StatusCode};
use actix_web::{
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web::{self, Data, Query},
    Error,
};

use futures_util::future::LocalBoxFuture;
use log::{error, warn};
use secp256k1::hashes::Hash;
use secp256k1::Message;
use secp256k1::{ecdsa::Signature, Secp256k1};
use secp256k1::{hashes::sha256, PublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ServerNonce, UnprotectedPaths};

pub struct Verifier;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AuthenticatedMessage {
    pub message: Value,
    pub public_key: String,
    pub signature: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AuthenticatedQueryParams {
    pub key: String, // the public key
    pub uuid: Option<String>,
    pub state: Option<String>,
    pub signature: String,
}

impl<S: 'static, B> Transform<S, ServiceRequest> for Verifier
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = VerifySignatureMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(VerifySignatureMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct VerifySignatureMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for VerifySignatureMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        let unprotected_paths = req
            .app_data::<Data<UnprotectedPaths>>()
            .expect("unable to get unprotected paths from app data");

        if unprotected_paths.paths.contains(&req.path().to_string()) {
            return Box::pin(async move { svc.call(req).await });
        }

        let nonces = req
            .app_data::<Data<Mutex<ServerNonce>>>()
            .expect("unable to get nonces from app data");
        let nonces = nonces
            .lock()
            .expect("unable to lock nonces mutex")
            .nonces
            .clone();

        let temp_headers = req.headers().clone();
        let auth_header_nonce = temp_headers.get("authorization");
        if auth_header_nonce.is_none() {
            warn!("did not find auth header in request");
            return Box::pin(async move {
                let mut res = svc.call(req).await?;
                *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                Ok(res)
            });
        };
        Box::pin(async move {
            let temp_headers = req.headers().clone();
            let auth_header_nonce = match temp_headers
                .get("authorization")
                .expect("to find the auth header")
                .to_str()
            {
                Ok(nonce) => nonce,
                Err(_) => {
                    warn!("could not convert auth header to string");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };

            if req.method() == actix_web::http::Method::GET {
                let query_params = req
                    .extract::<web::Query<AuthenticatedQueryParams>>()
                    .await
                    .expect("unable to extract query params");
                let mut res = svc.call(req).await?;

                if verify_query_params(query_params, auth_header_nonce).is_err()
                    || !nonces.contains(&auth_header_nonce.to_string())
                {
                    error!("Failed to verify signature or nonce");
                    *res.response_mut().status_mut() = StatusCode::FORBIDDEN;
                }
                Ok(res)
            } else {
                let body = req
                    .extract::<web::Bytes>()
                    .await
                    .expect("unable to extract body");

                let body_json = match serde_json::from_slice::<AuthenticatedMessage>(&body) {
                    Ok(body) => body,
                    Err(_) => {
                        error!("unable to parse body");
                        let mut res = svc.call(req).await?;
                        *res.response_mut().status_mut() = StatusCode::FORBIDDEN;
                        return Ok(res);
                    }
                };

                let message_nonce = &body_json.clone().message["nonce"];
                let message_nonce = match message_nonce.as_str() {
                    Some(nonce) => nonce,
                    None => {
                        error!("unable to parse nonce from body");
                        let mut res = svc.call(req).await?;
                        *res.response_mut().status_mut() = StatusCode::FORBIDDEN;
                        return Ok(res);
                    }
                };

                if verify_body(body_json).is_err()
                    || !nonces.contains(&auth_header_nonce.to_string())
                    || auth_header_nonce != message_nonce
                {
                    error!("Failed to verify signature or nonce");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = StatusCode::FORBIDDEN;
                    return Ok(res);
                }
                req.set_payload(bytes_to_payload(body));
                let res = svc.call(req).await?;
                Ok(res)
            }
        })
    }
}

fn verify_query_params(
    query_string: Query<AuthenticatedQueryParams>,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let sig: Signature = Signature::from_str(&query_string.signature)?;
    let message = Message::from(sha256::Hash::hash(message.as_bytes()));
    let pub_key = PublicKey::from_str(&query_string.key)?;

    let secp = Secp256k1::new();
    Ok(secp.verify_ecdsa(&message, &sig, &pub_key)?)
}

fn verify_body(body_json: AuthenticatedMessage) -> Result<(), Box<dyn std::error::Error>> {
    let sig: Signature = Signature::from_str(&body_json.signature)?;

    let message = Message::from(sha256::Hash::hash(body_json.message.to_string().as_bytes()));
    let pub_key = PublicKey::from_str(&body_json.public_key)?;

    let secp = Secp256k1::new();
    Ok(secp.verify_ecdsa(&message, &sig, &pub_key)?)
}

fn bytes_to_payload(buf: web::Bytes) -> dev::Payload {
    let (_, mut pl) = h1::Payload::create(true);
    pl.unread_data(buf);
    dev::Payload::from(pl)
}
