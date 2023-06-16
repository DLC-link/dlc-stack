#[macro_use]
extern crate log;
extern crate core;
use ::hex::ToHex;
use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer};

use clap::Parser;

use lightning::util::ser::{Readable, Writeable};

use secp256k1_zkp::rand::thread_rng;
use secp256k1_zkp::{
    hashes::*, All, KeyPair, Message, Secp256k1, SecretKey, XOnlyPublicKey as SchnorrPublicKey,
};
use std::{env, io::Cursor};

use serde::{Deserialize, Serialize};

use sled::IVec;
use std::path::PathBuf;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use sibyls::oracle::DbValue;

use dlc_messages::oracle_msgs::{
    DigitDecompositionEventDescriptor, EventDescriptor, OracleAnnouncement, OracleAttestation,
    OracleEvent,
};

mod error;
use error::SibylsError;
use sibyls::oracle::secret_key::get_or_generate_keypair;

mod oracle;
use oracle::Oracle;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SortOrder {
    Insertion,
    ReverseInsertion,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Filters {
    sort_by: SortOrder,
    page: u32,
    // asset_pair: AssetPair,
    maturation: String,
    outcome: Option<u64>,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            sort_by: SortOrder::ReverseInsertion,
            page: 0,
            // asset_pair: AssetPair::BTCUSD,
            maturation: "".to_string(),
            outcome: None,
        }
    }
}

#[derive(Serialize)]
struct ApiOracleEvent {
    event_id: String,
    uuid: String,
    rust_announcement_json: String,
    rust_announcement: String,
    rust_attestation_json: Option<String>,
    rust_attestation: Option<String>,
    maturation: String,
    outcome: Option<u64>,
}

fn parse_database_entry(event: IVec) -> ApiOracleEvent {
    let event: DbValue = serde_json::from_str(&String::from_utf8_lossy(&event)).unwrap();

    let announcement_vec = event.1.clone();
    let announcement = OracleAnnouncement::read(&mut Cursor::new(&announcement_vec)).unwrap();

    let db_att = event.2.clone();
    let decoded_att_json = match db_att {
        None => None,
        Some(att_vec) => {
            let mut attestation_cursor = Cursor::new(&att_vec);

            match OracleAttestation::read(&mut attestation_cursor) {
                Ok(att) => Some(format!("{:?}", att)),
                Err(_) => Some("Error decoding attestatoin".to_string()),
            }
        }
    };

    ApiOracleEvent {
        event_id: announcement.oracle_event.event_id.clone(),
        uuid: event.4,
        rust_announcement_json: serde_json::to_string(&announcement).unwrap(),
        rust_announcement: event.1.encode_hex::<String>(),
        rust_attestation_json: decoded_att_json,
        rust_attestation: event.2.map(|att| att.encode_hex::<String>()),
        maturation: announcement.oracle_event.event_maturity_epoch.to_string(),
        outcome: event.3,
    }
}

pub fn generate_nonces_for_event(
    secp: &Secp256k1<All>,
    event_descriptor: &EventDescriptor,
) -> (Vec<SchnorrPublicKey>, Vec<SecretKey>) {
    let nb_nonces = match event_descriptor {
        EventDescriptor::DigitDecompositionEvent(d) => d.nb_digits,
        EventDescriptor::EnumEvent(_) => panic!(),
    };

    let priv_nonces: Vec<_> = (0..nb_nonces)
        .map(|_| SecretKey::new(&mut thread_rng()))
        .collect();
    let key_pairs: Vec<_> = priv_nonces
        .iter()
        .map(|x| KeyPair::from_seckey_slice(secp, x.as_ref()).unwrap())
        .collect();

    let nonces = key_pairs
        .iter()
        .map(|k| SchnorrPublicKey::from_keypair(k).0)
        .collect();

    (nonces, priv_nonces)
}

pub fn build_announcement(
    keypair: &KeyPair,
    secp: &Secp256k1<All>,
    maturation: OffsetDateTime,
    event_id: String,
) -> Result<(OracleAnnouncement, Vec<SecretKey>), secp256k1_zkp::UpstreamError> {
    let event_descriptor =
        EventDescriptor::DigitDecompositionEvent(DigitDecompositionEventDescriptor {
            base: 2,
            is_signed: false,
            unit: "BTCUSD".to_string(),
            precision: 0,
            nb_digits: 14u16,
        });
    let (oracle_nonces, sk_nonces) = generate_nonces_for_event(secp, &event_descriptor);
    let oracle_event = OracleEvent {
        oracle_nonces,
        event_maturity_epoch: maturation.unix_timestamp().try_into().unwrap(),
        event_descriptor: event_descriptor.clone(),
        event_id: event_id.to_string(),
    };
    let mut event_hex = Vec::new();
    oracle_event
        .write(&mut event_hex)
        .expect("Error writing oracle event");
    let msg = Message::from_hashed_data::<secp256k1_zkp::hashes::sha256::Hash>(&event_hex);
    let sig = secp.sign_schnorr(&msg, keypair);
    let announcement = OracleAnnouncement {
        oracle_event,
        oracle_public_key: keypair.public_key().into(),
        announcement_signature: sig,
    };
    Ok((announcement, sk_nonces))
}

pub fn build_attestation(
    outstanding_sk_nonces: Vec<SecretKey>,
    key_pair: &KeyPair,
    secp: &Secp256k1<All>,
    outcomes: Vec<String>,
) -> OracleAttestation {
    let nonces = outstanding_sk_nonces;
    let signatures = outcomes
        .iter()
        .zip(nonces.iter())
        .map(|(x, nonce)| {
            let msg =
                Message::from_hashed_data::<secp256k1_zkp::hashes::sha256::Hash>(x.as_bytes());
            dlc::secp_utils::schnorrsig_sign_with_nonce(secp, &msg, key_pair, nonce.as_ref())
        })
        .collect();
    OracleAttestation {
        oracle_public_key: key_pair.public_key().into(),
        signatures,
        outcomes,
    }
}

#[get("/create_event/{uuid}")]
async fn create_event(
    oracle: web::Data<Oracle>,
    filters: web::Query<Filters>,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse, actix_web::Error> {
    info!("GET /create_event/{}: {:#?}", path, filters);
    let uuid = path.to_string();
    let maturation = OffsetDateTime::parse(&filters.maturation, &Rfc3339)
        .map_err(SibylsError::DatetimeParseError)?;

    info!(
        "Creating event for uuid:{} and maturation_time :{}",
        uuid, maturation
    );

    let (announcement_obj, outstanding_sk_nonces) =
        build_announcement(&oracle.key_pair, &oracle.secp, maturation, uuid.clone()).unwrap();

    let db_value = DbValue(
        Some(outstanding_sk_nonces),
        announcement_obj.encode(),
        None,
        None,
        uuid.clone(),
    );

    let new_event = serde_json::to_string(&db_value)?.into_bytes();
    info!("Inserting new event ...[uuid: {}]", uuid.clone());
    if oracle.event_handler.storage_api.is_some() {
        oracle
            .event_handler
            .storage_api
            .as_ref()
            .unwrap()
            .insert(uuid.clone(), new_event.clone())
            .await
            .unwrap();
    } else {
        oracle
            .event_handler
            .sled_db
            .as_ref()
            .unwrap()
            .insert(uuid.clone().into_bytes(), new_event.clone())
            .unwrap();
    }

    Ok(HttpResponse::Ok().json(parse_database_entry(new_event.into())))
}

#[get("/attest/{uuid}")]
async fn attest(
    oracle: web::Data<Oracle>,
    filters: web::Query<Filters>,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse, actix_web::Error> {
    info!("GET /attest/{}: {:#?}", path, filters);
    let uuid = path.to_string();
    let outcome = &filters.outcome.unwrap();

    if oracle.event_handler.is_empty() {
        info!("no oracle events found");
        return Err(SibylsError::OracleEventNotFoundError(uuid).into());
    }

    info!("retrieving oracle event with uuid {}", uuid);
    let mut event: DbValue;
    if oracle.event_handler.storage_api.is_some() {
        let event_vec = match oracle
            .event_handler
            .storage_api
            .as_ref()
            .unwrap()
            .get(uuid.clone())
            .await
            .unwrap()
        {
            Some(val) => val,
            None => return Err(SibylsError::OracleEventNotFoundError(uuid).into()),
        };
        event = serde_json::from_str(&String::from_utf8_lossy(&event_vec)).unwrap();
    } else {
        let event_ivec = match oracle
            .event_handler
            .sled_db
            .as_ref()
            .unwrap()
            .get(uuid.as_bytes())
            .map_err(SibylsError::DatabaseError)?
        {
            Some(val) => val,
            None => return Err(SibylsError::OracleEventNotFoundError(uuid).into()),
        };
        event = serde_json::from_str(&String::from_utf8_lossy(&event_ivec)).unwrap();
    }

    let outstanding_sk_nonces = event.clone().0.unwrap();

    let announcement = OracleAnnouncement::read(&mut Cursor::new(&event.1)).unwrap();

    let num_digits_to_sign = match announcement.oracle_event.event_descriptor {
        dlc_messages::oracle_msgs::EventDescriptor::DigitDecompositionEvent(e) => e.nb_digits,
        _ => {
            return Err(SibylsError::OracleEventNotFoundError(
                "Got an unexpected EventDescriptor type!".to_string(),
            )
            .into())
        }
    };

    // Here, we take the outcome of the DLC (0-10000), break it down into binary, break it into a vec of characters
    let outcomes = format!("{:0width$b}", outcome, width = num_digits_to_sign as usize)
        .chars()
        .map(|char| char.to_string())
        .collect::<Vec<_>>();

    let attestation = build_attestation(
        outstanding_sk_nonces,
        oracle.get_keypair(),
        &oracle.get_secp(),
        outcomes,
    );

    event.3 = Some(*outcome);
    event.2 = Some(attestation.encode());

    info!(
        "attesting with maturation {} and attestation {:#?}",
        path, attestation
    );

    let new_event = serde_json::to_string(&event)?.into_bytes();

    if oracle.event_handler.storage_api.is_some() {
        let _insert_event = match oracle
            .event_handler
            .storage_api
            .as_ref()
            .unwrap()
            .insert(path.clone(), new_event.clone())
            .await
            .unwrap()
        {
            Some(val) => val,
            None => return Err(SibylsError::OracleEventNotFoundError(uuid).into()),
        };
    } else {
        let _insert_event = match oracle
            .event_handler
            .sled_db
            .as_ref()
            .unwrap()
            .insert(path.clone().as_bytes(), new_event.clone())
            .map_err(SibylsError::DatabaseError)?
        {
            Some(val) => val,
            None => return Err(SibylsError::OracleEventNotFoundError(uuid).into()),
        };
    }
    Ok(HttpResponse::Ok().json(parse_database_entry(new_event.into())))
}

#[get("/announcements")]
async fn announcements(
    oracle: web::Data<Oracle>,
    filters: web::Query<Filters>,
) -> actix_web::Result<HttpResponse, actix_web::Error> {
    info!("GET /announcements: {:#?}", filters);
    if oracle.event_handler.is_empty() {
        info!("no oracle events found");
        return Ok(HttpResponse::Ok().json(Vec::<ApiOracleEvent>::new()));
    }
    if oracle.event_handler.storage_api.is_some() {
        return Ok(HttpResponse::Ok().json(
            oracle
                .event_handler
                .storage_api
                .as_ref()
                .unwrap()
                .get_all()
                .await
                .unwrap()
                .unwrap()
                .iter()
                .map(|result| parse_database_entry(result.clone().1.into()))
                .collect::<Vec<_>>(),
        ));
    } else {
        return Ok(HttpResponse::Ok().json(
            oracle
                .event_handler
                .sled_db
                .as_ref()
                .unwrap()
                .iter()
                .map(|result| parse_database_entry(result.unwrap().1))
                .collect::<Vec<_>>(),
        ));
    }
}

#[get("/announcement/{uuid}")]
async fn get_announcement(
    oracle: web::Data<Oracle>,
    filters: web::Query<Filters>,
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse, actix_web::Error> {
    info!("GET /announcement/{}: {:#?}", path, filters);
    let uuid = path.to_string();

    if oracle.event_handler.is_empty() {
        info!("no oracle events found");
        return Err(SibylsError::OracleEventNotFoundError(path.to_string()).into());
    }

    info!("retrieving oracle event with uuid {}", uuid);
    if oracle.event_handler.storage_api.is_some() {
        let event = match oracle
            .event_handler
            .storage_api
            .as_ref()
            .unwrap()
            .get(uuid.clone())
            .await
            .unwrap()
        {
            Some(val) => val,
            None => return Err(SibylsError::OracleEventNotFoundError(path.to_string()).into()),
        };
        Ok(HttpResponse::Ok().json(parse_database_entry(event.into())))
    } else {
        let event = match oracle
            .event_handler
            .sled_db
            .as_ref()
            .unwrap()
            .get(uuid.as_bytes())
            .map_err(SibylsError::DatabaseError)?
        {
            Some(val) => val,
            None => return Err(SibylsError::OracleEventNotFoundError(path.to_string()).into()),
        };
        Ok(HttpResponse::Ok().json(parse_database_entry(event)))
    }
}

#[get("/publickey")]
async fn publickey() -> actix_web::Result<HttpResponse, actix_web::Error> {
    info!("GET /publickey");
    let secp: Secp256k1<All> = Secp256k1::new();
    let key_pair = get_or_generate_keypair(&secp, Some(PathBuf::from("config/secret.key"))).await;
    let pubkey = SchnorrPublicKey::from_keypair(&key_pair).0;
    Ok(HttpResponse::Ok().json(pubkey))
}

#[derive(Parser)]
/// Simple DLC oracle implementation
struct Args {
    /// Optional private key file; if not provided, one is generated
    #[clap(short, long, parse(from_os_str), value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    secret_key_file: Option<std::path::PathBuf>,

    /// Optional asset pair config file; if not provided, it is assumed to exist at "config/asset_pair.json"
    #[clap(short, long, parse(from_os_str), value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    asset_pair_config_file: Option<std::path::PathBuf>,

    /// Optional oracle config file; if not provided, it is assumed to exist at "config/oracle.json"
    #[clap(short, long, parse(from_os_str), value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    oracle_config_file: Option<std::path::PathBuf>,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let secp = Secp256k1::new();
    let key_pair = get_or_generate_keypair(&secp, args.secret_key_file).await;
    info!(
        "oracle keypair successfully generated, pubkey is {}",
        key_pair.public_key().serialize().encode_hex::<String>()
    );

    // setup event databases
    let oracle = Oracle::new(key_pair, secp).unwrap();

    // setup and run server
    let port: u16 = env::var("ORACLE_PORT")
        .unwrap_or("8080".to_string())
        .parse()
        .unwrap_or(8080);
    info!("starting server on port {port}");
    HttpServer::new(move || {
        let cors = Cors::default();
        let cors = cors
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"])
            .max_age(3600);
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(oracle.clone()))
            .service(
                web::scope("/v1")
                    .service(announcements)
                    .service(get_announcement)
                    .service(publickey)
                    .service(attest)
                    .service(create_event),
            )
    })
    .bind(("0.0.0.0", port))?
    // .bind(("54.198.187.245", 8080))? //TODO: Should we bind to only certain IPs for security?
    .run()
    .await?;

    Ok(())
}
