// use actix_web::web::{Data, Json};
// use actix_web::{get, post, put, HttpResponse, Responder};
// use dlc_clients::OracleBackendClient;
// use log::{error, info};
// use rayon::prelude::*;
// use serde::{Deserialize, Serialize};
// use std::collections::{HashMap, HashSet};
// use std::sync::{Arc, Mutex};

// type Oracles = Arc<Mutex<HashMap<String, Oracle>>>;
// type UnverifiedOracles = Arc<Mutex<HashSet<String>>>;

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct Oracle {
//     pub public_key: String,
//     pub url: String,
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct OracleInput {
//     public_key: Option<String>,
//     url: String,
// }

// #[get("/ping")]
// pub async fn ping() -> impl Responder {
//     HttpResponse::Ok().body("pong")
// }

// #[get("/oracles")]
// pub async fn get_oracles(
//     oracles: Data<Oracles>,
//     url_replace_rules: Data<HashMap<String, String>>,
// ) -> impl Responder {
//     HttpResponse::Ok().json(convert_oracles(&oracles, &url_replace_rules))
// }

// #[get("/unverified_oracles")]
// pub async fn get_unverified_oracles(unverified_oracles: Data<UnverifiedOracles>) -> impl Responder {
//     HttpResponse::Ok().json(convert_unverified_oracles(&unverified_oracles))
// }

// #[post["/register"]]
// pub async fn register_oracle(
//     oracle: Json<OracleInput>,
//     unverified_oracles: Data<UnverifiedOracles>,
// ) -> impl Responder {
//     let oracle = oracle.into_inner();
//     info!(
//         "Register (unverified) oracle with url: {}",
//         oracle.url.clone()
//     );
//     let mut unverified_oracles = unverified_oracles.lock().unwrap();
//     unverified_oracles.insert(oracle.url.clone());
//     HttpResponse::Created()
// }

// #[put["/verify"]]
// pub async fn verify_oracle(
//     oracle: Json<OracleInput>,
//     oracles: Data<Oracles>,
//     unverified_oracles: Data<UnverifiedOracles>,
// ) -> impl Responder {
//     let oracle = oracle.into_inner();
//     info!("Verify oracle with url: {}", oracle.url.clone());
//     match OracleBackendClient::new(oracle.url.clone())
//         .get_public_key()
//         .await
//     {
//         Ok(key) => {
//             let url = oracle.url;
//             let verified_oracle = Oracle {
//                 public_key: key.clone(),
//                 url: url.clone(),
//             };
//             info!(
//                 "Verified key is '{}' for oracle with url '{}'",
//                 key.clone(),
//                 url.clone()
//             );
//             let mut oracles = oracles.lock().unwrap();
//             oracles.insert(key.clone(), verified_oracle);
//             let mut unverified_oracles = unverified_oracles.lock().unwrap();
//             unverified_oracles.remove(&url);
//             HttpResponse::Accepted()
//         }
//         Err(err) => {
//             error!("Error while calling get_public_key {:?}", err);
//             HttpResponse::InternalServerError()
//         }
//     }
// }

// fn convert_oracles(oracles: &Oracles, replace_rules: &HashMap<String, String>) -> Vec<Oracle> {
//     let oracles = oracles.lock().unwrap();
//     let oracles_vec: Vec<&Oracle> = oracles.values().collect();
//     oracles_vec
//         .into_par_iter()
//         .map(|oracle| {
//             let mut oracle = oracle.clone();
//             let url_to_check = &oracle.url;
//             for (match_string, replacement) in replace_rules {
//                 if url_to_check.contains(match_string) {
//                     let parts: Vec<&str> = replacement.split(":").collect();
//                     let new_url = format!("{}:{}", parts[0], parts[1]);
//                     oracle.url = new_url;
//                     break;
//                 }
//             }
//             Oracle {
//                 public_key: oracle.public_key.replace("\"", ""),
//                 url: oracle.url,
//             }
//         })
//         .collect()
// }

// fn convert_unverified_oracles(oracles: &UnverifiedOracles) -> Vec<String> {
//     let oracles = oracles.lock().unwrap();
//     oracles.iter().cloned().collect()
// }
