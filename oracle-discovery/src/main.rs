// mod oracles;

// extern crate log;
// use actix_web::web::Data;
// use actix_web::{error, web, App, HttpResponse, HttpServer};
// use dlc_clients::OracleBackendClient;
// use log::{info, warn};
// use oracles::{get_oracles, get_unverified_oracles, ping, register_oracle, verify_oracle, Oracle};
// use std::collections::{HashMap, HashSet};
// use std::env;
// use std::sync::{Arc, Mutex};

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     env_logger::init();
//     let port: u16 = env::var("ORACLE_DISCOVERY_PORT")
//         .unwrap_or("8400".to_string())
//         .parse()
//         .unwrap_or(8400);
//     let oracles = Arc::new(Mutex::new(HashMap::<String, Oracle>::new()));
//     let unverified_oracles = Arc::new(Mutex::new(HashSet::<String>::new()));
//     let oracle_urls = match env::var("ORACLE_URLS") {
//         Ok(urls) => urls
//             .split(",")
//             .map(|s| s.to_string())
//             .collect::<Vec<String>>(),
//         Err(_) => vec![],
//     };
//     let url_replace_rules = match env::var("URL_REPLACE_RULES") {
//         Ok(replace_rules_str) => replace_rules_str
//             .split(",")
//             .map(|s| {
//                 let parts: Vec<&str> = s.split("=").collect();
//                 (parts[0].to_string(), parts[1].to_string())
//             })
//             .collect(),
//         Err(_) => HashMap::new(),
//     };
//     for url in oracle_urls {
//         let res = OracleBackendClient::new(url.clone()).get_public_key().await;
//         match res {
//             Ok(pub_key) => {
//                 info!(
//                     "Getting public key {} for url {}",
//                     pub_key.clone(),
//                     url.clone()
//                 );
//                 let mut oracles = oracles.lock().unwrap();
//                 oracles.insert(
//                     pub_key.clone(),
//                     Oracle {
//                         public_key: pub_key.clone(),
//                         url: url.clone(),
//                     },
//                 );
//             }
//             Err(api_err) => {
//                 warn!("Error getting public key for url {}: {}", url, api_err);
//                 let mut u_oracles = unverified_oracles.lock().unwrap();
//                 u_oracles.insert(url);
//             }
//         }
//     }
//     HttpServer::new(move || {
//         App::new()
//             .app_data(Data::new(oracles.clone()))
//             .app_data(Data::new(unverified_oracles.clone()))
//             .app_data(Data::new(url_replace_rules.clone()))
//             .app_data(web::JsonConfig::default().error_handler(|err, _req| {
//                 error::InternalError::from_response(
//                     "",
//                     HttpResponse::BadRequest()
//                         .content_type("application/json")
//                         .body(format!(r#"{{"error":"{}"}}"#, err)),
//                 )
//                 .into()
//             }))
//             .service(
//                 web::scope("/v1")
//                     .service(ping)
//                     .service(register_oracle)
//                     .service(verify_oracle)
//                     .service(get_unverified_oracles)
//                     .service(get_oracles),
//             )
//     })
//     .bind(("0.0.0.0", port))?
//     .run()
//     .await
// }
