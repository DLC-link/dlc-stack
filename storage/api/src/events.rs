use crate::DbPool;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{delete, get, post, put, HttpResponse, Responder};
use dlc_storage_common::models::{DeleteEvent, EventRequestParams, NewEvent, UpdateEvent};
use dlc_storage_reader;
use dlc_storage_writer;
use log::debug;

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
    let events = dlc_storage_writer::update_event(&mut conn, event.into_inner()).unwrap();
    debug!("PUT: /events/ : {:?}", events);
    HttpResponse::Ok().json(events)
}

#[delete("/event")]
pub async fn delete_event(pool: Data<DbPool>, event: Json<DeleteEvent>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let delete_event = event.into_inner();
    let num_deleted = dlc_storage_writer::delete_event(&mut conn, delete_event.clone()).unwrap();
    debug!("DELETE: /events - {:?} : {:?}", delete_event, num_deleted);
    HttpResponse::Ok().json(num_deleted)
}

#[delete("/events/{ckey}")]
pub async fn delete_events(pool: Data<DbPool>, ckey: Path<String>) -> impl Responder {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let num_deleted = dlc_storage_writer::delete_events(&mut conn, &ckey).unwrap();
    debug!("DELETE: /events key={} : {:?}", ckey, num_deleted);
    HttpResponse::Ok().json(num_deleted)
}
