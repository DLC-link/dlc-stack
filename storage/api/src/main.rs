mod contracts;
mod events;

use actix_cors::Cors;
use contracts::*;
use events::*;
extern crate log;
use crate::events::get_events;
use actix_web::web::Data;
use actix_web::{error, get, web, App, HttpResponse, HttpServer, Responder};
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use dlc_storage_writer::apply_migrations;
use dotenv::dotenv;
use serde_json::json;
use std::env;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[get("/health")]
pub async fn get_health() -> impl Responder {
    HttpResponse::Ok().json(json!({"data": [{"status": "healthy", "message": ""}]}))
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
        .unwrap();
    if migrate {
        apply_migrations(&mut conn);
    }
    //TODO: change allow_any_origin / allow_any_header / allow_any_method to something more restrictive
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method()
            .max_age(3600);
        App::new()
            .wrap(cors)
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
