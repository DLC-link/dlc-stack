use std::{
    future::{ready, Ready},
    rc::Rc,
    str::FromStr,
    sync::Mutex,
};

use actix_http::h1;
use actix_web::{
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    web::{self, Data},
    Error,
};

use futures_util::future::LocalBoxFuture;
use log::error;
use secp256k1::hashes::Hash;
use secp256k1::Message;
use secp256k1::{ecdsa::Signature, Secp256k1};
use secp256k1::{hashes::sha256, PublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ServerNonce, UnprotectedPaths};

pub struct Verifier;

#[derive(Deserialize, Debug, Serialize)]
pub struct AuthenticatedMessage {
    pub message: Value,
    pub public_key: String,
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
            return Box::pin(async move {
                let res = svc.call(req).await?;
                Ok(res)
            });
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
            return Box::pin(async move {
                let mut res = svc.call(req).await?;
                *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                Ok(res)
            });
        };
        Box::pin(async move {
            let temp_headers = req.headers().clone();
            let auth_header_nonce = match temp_headers.get("authorization") {
                Some(nonce) => nonce,
                None => {
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };
            let auth_header_nonce = match auth_header_nonce.to_str() {
                Ok(nonce) => nonce,
                Err(_) => {
                    error!("did not find auth header in request");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };

            let body = req
                .extract::<web::Bytes>()
                .await
                .expect("unable to extract body");

            let body_json = match serde_json::from_slice::<AuthenticatedMessage>(&body) {
                Ok(body_json) => body_json,
                Err(_) => {
                    error!("unable to parse body as json");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };

            let secp = Secp256k1::new();

            let sig: Signature = match Signature::from_str(&body_json.signature) {
                Ok(sig) => sig,
                Err(_) => {
                    error!("unable to parse signature");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };
            let message =
                Message::from(sha256::Hash::hash(body_json.message.to_string().as_bytes()));
            let pub_key = match PublicKey::from_str(&body_json.public_key) {
                Ok(pub_key) => pub_key,
                Err(_) => {
                    error!("unable to parse public key");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };

            let message_nonce = match body_json.message["nonce"].as_str() {
                Some(nonce) => nonce,
                None => {
                    error!("unable to parse nonce from message");
                    let mut res = svc.call(req).await?;
                    *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
                    return Ok(res);
                }
            };
            req.set_payload(bytes_to_payload(body));
            let mut res = svc.call(req).await?;

            if secp.verify_ecdsa(&message, &sig, &pub_key).is_err()
                || !nonces.contains(&auth_header_nonce.to_string())
                || auth_header_nonce != message_nonce
            {
                error!("Failed to verify signature or nonce");
                *res.response_mut().status_mut() = actix_web::http::StatusCode::FORBIDDEN;
            }
            Ok(res)
        })
    }
}

fn bytes_to_payload(buf: web::Bytes) -> dev::Payload {
    let (_, mut pl) = h1::Payload::create(true);
    pl.unread_data(buf);
    dev::Payload::from(pl)
}
