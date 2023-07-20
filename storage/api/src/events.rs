use crate::DbPool;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, post, put, HttpResponse, Responder};
use dlc_storage_common::models::{DeleteEvent, EventRequestParams, NewEvent, UpdateEvent};
use dlc_storage_reader;
use dlc_storage_writer;
use log::{debug, warn};
use serde_json::json;

#[get("/events")]
pub async fn get_events(
    pool: Data<DbPool>,
    event_params: web::Query<EventRequestParams>,
) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let events = dlc_storage_reader::get_events(&mut conn, event_params.into_inner()).unwrap();
    debug!("GET: /events : {:?}", events);
    HttpResponse::Ok().json(events)
}

#[post("/events")]
pub async fn create_event(pool: Data<DbPool>, event: Json<NewEvent>) -> impl Responder {
    debug!("POST: /events : {:?}", event);
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    match dlc_storage_writer::create_event(&mut conn, event.into_inner()) {
        Ok(event) => HttpResponse::Ok().json(event),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[put("/events")]
pub async fn update_event(pool: Data<DbPool>, event: Json<UpdateEvent>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_updated = match dlc_storage_writer::update_event(&mut conn, event.into_inner()) {
        Ok(num_updated) => num_updated,
        Err(e) => {
            warn!("Error updating event: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };
    match num_updated {
        0 => HttpResponse::NotFound().body("No event found"),
        _ => HttpResponse::Ok().json(json!({ "effected_num": num_updated })),
    }
}

#[delete("/event")]
pub async fn delete_event(pool: Data<DbPool>, event: Json<DeleteEvent>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = match dlc_storage_writer::delete_event(&mut conn, event.into_inner()) {
        Ok(num_deleted) => num_deleted,
        Err(e) => {
            warn!("Error deleting event: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };
    match num_deleted {
        0 => HttpResponse::NotFound().body("No event found"),
        _ => HttpResponse::Ok().json(json!({ "effected_num": num_deleted })),
    }
}

#[delete("/events/{ckey}")]
pub async fn delete_events(pool: Data<DbPool>, ckey: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_events(&mut conn, &ckey).unwrap();
    HttpResponse::Ok().json(json!({ "effected_num": num_deleted }))
}
