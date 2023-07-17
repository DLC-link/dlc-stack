mod contracts;
mod events;

use contracts::*;
use events::*;
extern crate log;
use crate::events::get_events;
use actix_web::web::Data;
use actix_web::{error, web, App, HttpResponse, HttpServer};
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use dlc_storage_writer::apply_migrations;
use dotenv::dotenv;
use std::env;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

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
    HttpServer::new(move || {
        App::new()
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
