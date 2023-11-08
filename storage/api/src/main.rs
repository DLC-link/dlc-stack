#![deny(clippy::unwrap_used)]
#![deny(unused_mut)]
#![deny(dead_code)]
mod contracts;
mod events;
mod verify_sigs;

use actix_cors::Cors;
use contracts::*;
use events::*;
use rand::distributions::{Alphanumeric, DistString};
use secp256k1::rand;
extern crate log;
use crate::events::get_events;
use actix_web::dev::Service as _;
use actix_web::web::Data;
use actix_web::{error, get, web, App, HttpResponse, HttpServer, Responder};
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use dlc_storage_writer::apply_migrations;
use dotenv::dotenv;
use serde_json::json;
use std::env;
use std::sync::Mutex;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

const NONCE_VEC_LENGTH: usize = 100;

#[get("/health")]
pub async fn get_health() -> impl Responder {
    HttpResponse::Ok().json(json!({"data": [{"status": "healthy", "message": ""}]}))
}

#[get("/request_nonce")]
pub async fn request_nonce(server_nonces: Data<Mutex<ServerNonce>>) -> impl Responder {
    let mut server_nonce_vec = server_nonces.lock().expect("Failed to lock nonce vec");
    while server_nonce_vec.nonces.len() >= NONCE_VEC_LENGTH {
        server_nonce_vec.nonces.remove(0); // remove the oldest
    }
    let random_nonce = Alphanumeric.sample_string(&mut rand::thread_rng(), 20);
    server_nonce_vec.nonces.push(random_nonce.to_string());
    HttpResponse::Ok().body(random_nonce.to_string())
}

#[derive(Debug, Clone)]
struct ServerNonce {
    nonces: Vec<String>,
}

#[derive(Debug)]
struct UnprotectedPaths {
    paths: Vec<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().ok();
    // e.g.: DATABASE_URL=postgresql://postgres:changeme@localhost:5432/postgres
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let mut conn = pool.get().expect("Failed to get connection from pool");
    let migrate: bool = env::var("MIGRATE")
        .unwrap_or("false".to_string())
        .parse()
        .expect("Missing required env var MIGRATE");
    if migrate {
        apply_migrations(&mut conn);
    }
    let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
    let unprotected_paths = Data::new(UnprotectedPaths {
        paths: vec!["/health".to_string(), "/request_nonce".to_string()],
    });

    //TODO: change allow_any_origin / allow_any_header / allow_any_method to something more restrictive
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method()
            .max_age(3600);
        App::new()
            .wrap(cors)
            .app_data(nonces.clone())
            .app_data(unprotected_paths.clone())
            .app_data(Data::new(pool.clone()))
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                error::InternalError::from_response(
                    "",
                    HttpResponse::BadRequest()
                        .content_type("application/json")
                        .body(format!(r#"{{"error":"{}"}}"#, err)),
                )
                .into()
            }))
            .wrap_fn(|req, srv| {
                let header_nonce = req.headers().get("authorization");
                if let Some(header_nonce) = header_nonce {
                    req.app_data::<Data<Mutex<ServerNonce>>>()
                        .expect("Failed to get nonces from app data")
                        .lock()
                        .expect("Failed to lock nonce vec")
                        .nonces
                        .retain(|n| n != header_nonce);
                }
                srv.call(req)
            })
            .wrap(verify_sigs::Verifier)
            .service(request_nonce)
            .service(get_health)
            .service(get_contracts)
            .service(create_contract)
            .service(update_contract)
            .service(delete_contract)
            .service(delete_contracts)
            .service(get_events)
            .service(create_event)
            .service(update_event)
            .service(delete_event)
            .service(delete_events)
    })
    .bind("0.0.0.0:8100")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use actix_http::header;
    use actix_web::{
        body::to_bytes,
        dev::Service,
        http::{Method, StatusCode},
        test::{self, init_service, TestRequest},
        web::Bytes,
        App, Error,
    };

    use secp256k1::hashes::sha256;
    use secp256k1::rand::rngs::OsRng;
    use secp256k1::Message;
    use secp256k1::{hashes::Hash, Secp256k1};

    use serde_json::Value;

    use super::*;

    trait BodyTest {
        fn as_str(&self) -> &str;
    }

    impl BodyTest for Bytes {
        fn as_str(&self) -> &str {
            std::str::from_utf8(self).expect("Failed to convert bytes to string")
        }
    }

    #[actix_web::test]
    async fn test_without_auth() -> Result<(), Error> {
        let app = init_service(App::new().service(get_health)).await;
        let req = TestRequest::default()
            .method(Method::GET)
            .uri("/health")
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        assert_eq!(
            serde_json::from_str::<Value>(body.as_str()).expect("Failed to parse json"),
            json!({"data": [{"status": "healthy", "message": ""}]}),
        );

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_good_auth() -> Result<(), Error> {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        let nonce = body.as_str();

        let new_contract = json!({
            "nonce": nonce,
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        let digest = Message::from(sha256::Hash::hash(new_contract.to_string().as_bytes()));
        let sig = secp.sign_ecdsa(&digest, &secret_key);
        assert!(secp.verify_ecdsa(&digest, &sig, &public_key).is_ok());

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key.to_string(),
            "signature": sig.to_string(),
        });

        let req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let res = test::call_service(&app, req).await;

        // It's not great to expect a 500 in a test, but in this case
        // it means it got to the function and attempted to interact with the DB
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR); // this means it worked

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_missing_sig() -> Result<(), Error> {
        let secp = Secp256k1::new();
        let (_secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        let nonce = body.as_str();

        let new_contract = json!({
            "nonce": nonce,
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key.to_string(),
        });

        let req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_missing_nonce_in_message_body() -> Result<(), Error> {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        let nonce = body.as_str();

        let new_contract = json!({
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        let digest = Message::from(sha256::Hash::hash(new_contract.to_string().as_bytes()));
        let sig = secp.sign_ecdsa(&digest, &secret_key);
        assert!(secp.verify_ecdsa(&digest, &sig, &public_key).is_ok());

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key.to_string(),
            "signature": sig.to_string(),
        });

        let req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let res = test::call_service(&app, req).await;

        // It's not great to expect a 500 in a test, but in this case
        // it means it got to the function and attempted to interact with the DB
        assert_eq!(res.status(), StatusCode::FORBIDDEN); // this means it worked

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_missing_nonce_in_header() -> Result<(), Error> {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        let nonce = body.as_str();

        let new_contract = json!({
            "nonce": nonce,
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        let digest = Message::from(sha256::Hash::hash(new_contract.to_string().as_bytes()));
        let sig = secp.sign_ecdsa(&digest, &secret_key);
        assert!(secp.verify_ecdsa(&digest, &sig, &public_key).is_ok());

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key.to_string(),
            "signature": sig.to_string(),
        });

        let req = TestRequest::default()
            .method(Method::POST)
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let res = test::call_service(&app, req).await;

        // It's not great to expect a 500 in a test, but in this case
        // it means it got to the function and attempted to interact with the DB
        assert_eq!(res.status(), StatusCode::FORBIDDEN); // this means it worked

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_bad_nonce() -> Result<(), Error> {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);

        // hardcoded bad nonce
        let nonce = "12345";

        let new_contract = json!({
            "nonce": nonce,
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        let digest = Message::from(sha256::Hash::hash(new_contract.to_string().as_bytes()));
        let sig = secp.sign_ecdsa(&digest, &secret_key);
        assert!(secp.verify_ecdsa(&digest, &sig, &public_key).is_ok());

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key.to_string(),
            "signature": sig.to_string(),
        });

        let req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_previously_used_nonce() -> Result<(), Error> {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        let nonce = body.as_str();

        let new_contract = json!({
            "nonce": nonce,
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key.to_string(),
        });

        let digest = Message::from(sha256::Hash::hash(new_contract.to_string().as_bytes()));
        let sig = secp.sign_ecdsa(&digest, &secret_key);
        assert!(secp.verify_ecdsa(&digest, &sig, &public_key).is_ok());

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key.to_string(),
            "signature": sig.to_string(),
        });

        let first_req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body.clone())
            .to_request();

        let second_req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let first_res = test::call_service(&app, first_req).await;

        let second_res = test::call_service(&app, second_req).await;

        // It's not great to expect a 500 in a test, but in this case
        // it means it got to the function and attempted to interact with the DB
        assert_eq!(first_res.status(), StatusCode::INTERNAL_SERVER_ERROR);

        assert_eq!(second_res.status(), StatusCode::FORBIDDEN);

        Ok(())
    }

    #[actix_web::test]
    async fn test_with_bad_sig() -> Result<(), Error> {
        //Signing the message with privkey1, but sending pubkey_2 in the body
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let (_secret_key_2, public_key_2) = secp.generate_keypair(&mut OsRng);
        let nonces = Data::new(Mutex::new(ServerNonce { nonces: vec![] }));
        let unprotected_paths = Data::new(UnprotectedPaths {
            paths: vec!["/health".to_string(), "/request_nonce".to_string()],
        });
        let app = init_service(
            App::new()
                .app_data(nonces.clone())
                .app_data(unprotected_paths.clone())
                .wrap_fn(|req, srv| {
                    let header_nonce = req.headers().get("authorization");
                    if let Some(header_nonce) = header_nonce {
                        req.app_data::<Data<Mutex<ServerNonce>>>()
                            .expect("Failed to get nonces from app data")
                            .lock()
                            .expect("Failed to unlock nonce vec")
                            .nonces
                            .retain(|x| x != header_nonce);
                    }
                    srv.call(req)
                })
                .wrap(verify_sigs::Verifier)
                .service(request_nonce)
                .service(create_contract),
        )
        .await;

        let nonce_request = TestRequest::default()
            .method(Method::GET)
            .uri("/request_nonce")
            .to_request();

        let res = test::call_service(&app, nonce_request).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body()).await.expect("Failed to get body");
        let nonce = body.as_str();

        let new_contract = json!({
            "nonce": nonce,
            "uuid": "123".to_string(),
            "state": "123".to_string(),
            "content": "123".to_string(),
            "key": public_key_2.to_string(),
        });

        let digest = Message::from(sha256::Hash::hash(new_contract.to_string().as_bytes()));
        let sig = secp.sign_ecdsa(&digest, &secret_key);
        assert!(secp.verify_ecdsa(&digest, &sig, &public_key).is_ok());

        let message_body = json!({
            "message": new_contract,
            "public_key": public_key_2.to_string(),
            "signature": sig.to_string(),
        });

        let req = TestRequest::default()
            .method(Method::POST)
            .insert_header((header::AUTHORIZATION, nonce))
            .uri("/contracts")
            .set_json(message_body)
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        Ok(())
    }
}
